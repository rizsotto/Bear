/*  Copyright (C) 2012-2020 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#include "config.h"

#include "Application.h"
#include "Session.h"
#include "er.h"
#include "libexec.h"
#include "libsys/Environment.h"
#include <libsys/FileSystem.h>
#include <libsys/Os.h>
#include <libsys/Process.h>

#include <functional>
#include <numeric>
#include <unistd.h>

#include <spdlog/spdlog.h>

namespace {

    rust::Result<ic::Session::HostInfo> create_host_info(const sys::Os& os)
    {
        return os.get_uname()
#ifdef HAVE_CS_PATH
            .map<ic::Session::HostInfo>([&os](auto result) {
                os.get_confstr(_CS_PATH)
                    .map<int>([&result](auto value) {
                        result.insert({ "_CS_PATH", value });
                        return 0;
                    });
                return result;
            })
#endif
#ifdef HAVE_CS_GNU_LIBC_VERSION
            .map<ic::Session::HostInfo>([&os](auto result) {
                os.get_confstr(_CS_GNU_LIBC_VERSION)
                    .map<int>([&result](auto value) {
                        result.insert({ "_CS_GNU_LIBC_VERSION", value });
                        return 0;
                    });
                return result;
            })
#endif
#ifdef HAVE__CS_GNU_LIBPTHREAD_VERSION
            .map<ic::Session::HostInfo>([&os](auto result) {
                os.get_confstr(__CS_GNU_LIBPTHREAD_VERSION)
                    .map<int>([&result](auto value) {
                        result.insert({ "__CS_GNU_LIBPTHREAD_VERSION", value });
                        return 0;
                    });
                return result;
            })
#endif
            .map_err<std::runtime_error>([](auto error) {
                return std::runtime_error("failed to get host info.");
            });
    }
}

namespace env {

    constexpr char GLIBC_PRELOAD_KEY[] = "LD_PRELOAD";

    using env_t = std::map<std::string, std::string>;
    using mapper_t = std::function<std::string(const std::string&, const std::string&)>;

    std::string
    merge_into_paths(const std::string& current, const std::string& value) noexcept
    {
        auto paths = sys::FileSystem::split_path(current);
        if (std::find(paths.begin(), paths.end(), value) == paths.end()) {
            paths.emplace_front(value);
            return sys::FileSystem::join_path(paths);
        } else {
            return current;
        }
    }

    void insert_or_assign(env_t& target, const char* key, const std::string& value) noexcept
    {
        if (auto it = target.find(key); it != target.end()) {
            it->second = value;
        } else {
            target.emplace(key, value);
        }
    }

    void insert_or_merge(
        env_t& target,
        const char* key,
        const std::string& value,
        const mapper_t& merger) noexcept
    {
        if (auto it = target.find(key); it != target.end()) {
            it->second = merger(it->second, value);
        } else {
            target.emplace(key, value);
        }
    }
}

namespace {

    class LibraryPreloadSession : public ic::Session {
    public:
        LibraryPreloadSession(HostInfo&& host_info, const std::string_view& library, const std::string_view& executor);

    public:
        [[nodiscard]] rust::Result<std::string_view> resolve(const std::string& name) const override;
        [[nodiscard]] rust::Result<std::map<std::string, std::string>> update(const std::map<std::string, std::string>& env) const override;
        [[nodiscard]] rust::Result<int> supervise(const std::vector<std::string_view>& command) const override;

        void set_server_address(const std::string&) override;

        [[nodiscard]] const HostInfo& get_host_info() const override;
        [[nodiscard]] std::string get_session_type() const override;

    private:
        HostInfo host_info_;
        std::string server_address_;
        std::string library_;
        std::string executor_;
    };

    LibraryPreloadSession::LibraryPreloadSession(ic::Session::HostInfo&& host_info, const std::string_view& library, const std::string_view& executor)
            : host_info_(host_info)
            , server_address_()
            , library_(library)
            , executor_(executor)
    {
        spdlog::debug("Created library preload session. [library={0}, executor={1}]", library_, executor_);
    }

    rust::Result<std::string_view> LibraryPreloadSession::resolve(const std::string& name) const
    {
        // The method has to be MT safe!!!
        return rust::Err(std::runtime_error("The session does not support resolve."));
    }

    rust::Result<std::map<std::string, std::string>> LibraryPreloadSession::update(const std::map<std::string, std::string>& env) const
    {
        // The method has to be MT safe!!!
        std::map<std::string, std::string> copy(env);
        env::insert_or_assign(copy, el::env::KEY_REPORTER, executor_);
        env::insert_or_assign(copy, el::env::KEY_DESTINATION, server_address_);
        env::insert_or_assign(copy, el::env::KEY_LIBRARY, library_); // TODO: deprecate it
        env::insert_or_merge(copy, env::GLIBC_PRELOAD_KEY, library_, env::merge_into_paths);

        return rust::Ok(copy);
    }

    rust::Result<int> LibraryPreloadSession::supervise(const std::vector<std::string_view>& command) const
    {
        auto environment = update(sys::env::from(const_cast<const char**>(environ)));
        auto program = sys::Os().get_path().and_then<std::string>([&command](auto path) {
            return sys::FileSystem().find_in_path(std::string(command.front()), path);
        });

        sys::Process process;
        return rust::merge(program, environment)
            .and_then<pid_t>([&command, &process, this](auto pair) {
                const auto& [program, environment] = pair;
                // create the argument list
                std::vector<const char*> args = {
                    executor_.c_str(),
                    er::flags::DESTINATION,
                    server_address_.c_str(),
                    er::flags::LIBRARY, // TODO: deprecate it
                    library_.c_str(), // TODO: deprecate it
                    er::flags::EXECUTE,
                    program.c_str(),
                    er::flags::COMMAND
                };
                std::transform(command.begin(), command.end(), std::back_insert_iterator(args),
                    [](const auto& it) { return it.data(); });
                // create environment pointer
                sys::env::Guard guard(environment);
                return process.spawn(executor_.c_str(), args.data(), guard.data());
            })
            .and_then<int>([&process](auto pid) {
                return process.wait_pid(pid);
            });
    }

    void LibraryPreloadSession::set_server_address(const std::string& value)
    {
        server_address_ = value;
    }

    const ic::Session::HostInfo& LibraryPreloadSession::get_host_info() const
    {
        return host_info_;
    }

    std::string LibraryPreloadSession::get_session_type() const
    {
        return std::string("library preload");
    }
}

namespace ic {

    rust::Result<Session::SharedPtr> Session::from(const flags::Arguments& args)
    {
        sys::Os os;
        auto host_info = create_host_info(os)
                             .unwrap_or_else([](auto error) {
                                 spdlog::info(error.what());
                                 return std::map<std::string, std::string>();
                             });

        auto library = args.as_string(ic::Application::LIBRARY);
        auto executor = args.as_string(ic::Application::EXECUTOR);

        return merge(library, executor)
            .map<Session::SharedPtr>([&host_info](auto pair) {
                const auto& [library, executor] = pair;
                auto result = new LibraryPreloadSession(std::move(host_info), library, executor);
                return std::shared_ptr<Session>(result);
            });
    }
}
