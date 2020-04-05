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

#include "Session.h"
#include "Application.h"
#include "libexec.h"

#ifdef HAVE_SYS_UTSNAME_H
#include <sys/utsname.h>
#endif
#include <functional>
#include <numeric>
#include <unistd.h>

#include <spdlog/spdlog.h>

namespace {

    constexpr char GLIBC_PRELOAD_KEY[] = "LD_PRELOAD";

    using env_t = std::map<std::string, std::string>;
    using mapper_t = std::function<std::string(const std::string&, const std::string&)>;

    std::list<std::string>
    split(const std::string& input, const char sep) noexcept
    {
        std::list<std::string> result;

        std::string::size_type previous = 0;
        do {
            const std::string::size_type current = input.find(sep, previous);
            result.emplace_back(input.substr(previous, current - previous));
            previous = (current != std::string::npos) ? current + 1 : current;
        } while (previous != std::string::npos);

        return result;
    }

    std::string
    merge_into_paths(const std::string& current, const std::string& value) noexcept
    {
        auto paths = split(current, ':');
        if (std::find(paths.begin(), paths.end(), value) == paths.end()) {
            paths.emplace_front(value);
            return std::accumulate(paths.begin(),
                                   paths.end(),
                                   std::string(),
                                   [](std::string acc, std::string item) {
                                       return (acc.empty()) ? item : acc + ':' + item;
                                   });
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

    void insert_or_merge(env_t& target,
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

#ifdef HAVE_CONFSTR
    rust::Result<std::string> get_confstr(const int key)
    {
        if (const size_t buffer_size = confstr(key, nullptr, 0); buffer_size != 0) {
            char buffer[buffer_size];
            if (const size_t size = confstr(key, buffer, buffer_size); size != 0) {
                return rust::Ok(std::string(buffer));
            }
        }
        return rust::Err(std::runtime_error("confstr failed."));
    }
#endif

    rust::Result<ic::Session::HostInfo> create_host_info()
    {
        std::map<std::string, std::string> result;
#ifdef HAVE_UNAME
        utsname name;
        if (const int status = uname(&name); status >= 0) {
            result.insert({ "sysname", std::string(name.sysname) });
            result.insert({ "release", std::string(name.release) });
            result.insert({ "version", std::string(name.version) });
            result.insert({ "machine", std::string(name.machine) });
        }
#endif
#ifdef HAVE_CONFSTR
#ifdef HAVE_CS_PATH
        get_confstr(_CS_PATH)
            .map<int>([&result](auto value) {
                result.insert({ "_CS_PATH", value });
                return 0;
            });
#endif
#ifdef HAVE_CS_GNU_LIBC_VERSION
        get_confstr(_CS_GNU_LIBC_VERSION)
            .map<int>([&result](auto value) {
                result.insert({ "_CS_GNU_LIBC_VERSION", value });
                return 0;
            });
#endif
#ifdef HAVE__CS_GNU_LIBPTHREAD_VERSION
        get_confstr(__CS_GNU_LIBPTHREAD_VERSION)
            .map<int>([&result](auto value) {
                result.insert({ "__CS_GNU_LIBPTHREAD_VERSION", value });
                return 0;
            });
#endif
#endif
        return (result.empty())
            ? rust::Result<ic::Session::HostInfo>(rust::Err(std::runtime_error("failed to get host info.")))
            : rust::Result<ic::Session::HostInfo>(rust::Ok(result));
    }

    class LibraryPreloadSession : public ic::Session {
    public:
        LibraryPreloadSession(HostInfo &&host_info, const std::string_view& library, const std::string_view& executor);

    public:
        [[nodiscard]] rust::Result<std::string_view> resolve(const std::string& name) const override;
        [[nodiscard]] rust::Result<std::map<std::string, std::string>> update(const env_t& env) const override;
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
    }

    rust::Result<std::string_view> LibraryPreloadSession::resolve(const std::string& name) const
    {
        return rust::Err(std::runtime_error("The session does not support resolve."));
    }

    rust::Result<std::map<std::string, std::string>> LibraryPreloadSession::update(const env_t& env) const
    {
        env_t copy(env);
        insert_or_assign(copy, el::env::KEY_REPORTER, executor_);
        insert_or_assign(copy, el::env::KEY_DESTINATION, server_address_);
        insert_or_assign(copy, el::env::KEY_LIBRARY, library_);
        insert_or_merge(copy, GLIBC_PRELOAD_KEY, library_, merge_into_paths);

        return rust::Ok(copy);
    }

    rust::Result<int> LibraryPreloadSession::supervise(const std::vector<std::string_view>& command) const
    {
        // TODO
        return rust::Ok(0);
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
        auto host_info = create_host_info()
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
