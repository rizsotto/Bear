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

#ifdef HAVE_SYS_UTSNAME_H
#include <sys/utsname.h>
#endif
#include <spawn.h>
#include <sys/types.h>
#include <sys/wait.h>

#include <cerrno>
#include <filesystem>
#include <functional>
#include <numeric>
#include <unistd.h>

#include <spdlog/spdlog.h>

namespace {

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
        utsname name = utsname {};
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
}

namespace env {

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

namespace start {

    std::map<std::string, std::string> to_map(const char** const input) noexcept
    {
        std::map<std::string, std::string> result;
        if (input == nullptr)
            return result;

        for (const char** it = input; *it != nullptr; ++it) {
            const auto end = *it + std::strlen(*it);
            const auto sep = std::find(*it, end, '=');
            const std::string key = (sep != end) ? std::string(*it, sep) : std::string(*it, end);
            const std::string value = (sep != end) ? std::string(sep + 1, end) : std::string();
            result.emplace(key, value);
        }
        return result;
    }

    //    char**
    //    to_c_array(const std::map<std::string, std::string>& input)
    //    {
    //        const size_t result_size = input.size() + 1;
    //        const auto result = new char*[result_size];
    //        auto result_it = result;
    //        for (const auto& it : input) {
    //            const size_t entry_size = it.first.size() + it.second.size() + 2;
    //            auto entry = new char[entry_size];
    //
    //            auto key = std::copy(it.first.begin(), it.first.end(), entry);
    //            *key++ = '=';
    //            auto value = std::copy(it.second.begin(), it.second.end(), key);
    //            *value = '\0';
    //
    //            *result_it++ = entry;
    //        }
    //        *result_it = nullptr;
    //        return result;
    //    }

    rust::Result<std::string> get_path()
    {
        if (auto env = getenv("PATH"); env != nullptr) {
            return rust::Ok(std::string(env));
        }
#ifdef HAVE_CONFSTR
        return get_confstr(_CS_PATH);
#else
        return rust::Err(std::runtime_error("Could not find PATH in environment"));
#endif
    }

    // TODO: validate if this is the right logic for execvp
    rust::Result<std::filesystem::path> executable_name(const std::string_view& program)
    {
        namespace fs = std::filesystem;
        constexpr fs::perms any_exec = fs::perms::owner_exec | fs::perms::group_exec | fs::perms::others_exec;

        // first check if the given program name is absolute.
        if (fs::path program_as_path = program; program_as_path.is_absolute()) {
            return rust::Ok(program_as_path);
        } else {
            // otherwise take the PATH environment directories and check if there are available
            // executable program with this name.
            return get_path()
                .and_then<std::filesystem::path>([&program](const auto& path) {
                    // the PATH is a list of directories separated by a colon character.
                    const std::list<std::string> directories = env::split(std::string(path), ':');
                    for (const auto& directory : directories) {
                        // any file which exists with the desired permission is the result.
                        fs::path candidate = fs::path(directory).append(program);
                        if ((fs::exists(candidate))
                            && (fs::is_regular_file(candidate))
                            && ((fs::status(candidate).permissions() & any_exec) != fs::perms::none)) {
                            return rust::Result<std::filesystem::path>(rust::Ok(candidate));
                        }
                    }
                    return rust::Result<std::filesystem::path>(rust::Err(std::runtime_error(
                        fmt::format("Could not find executable: {0}", program))));
                });
        }
    }

    rust::Result<pid_t> spawn(
        const std::filesystem::path& path,
        const std::vector<std::string_view> args,
        const std::map<std::string, std::string>& environment)
    {
        // TODO
        errno = ENOENT;
        pid_t child;
        //        if (0 != posix_spawn(&child, path.c_str(), nullptr, nullptr, const_cast<char**>(args), const_cast<char**>(envp))) {
        const auto message = fmt::format("posix_spawn system call failed. [errno: {0}]", errno);
        return rust::Err(std::runtime_error(message));
        //        } else {
        //            return rust::Ok(child);
        //        }
    }

    rust::Result<int> wait(const pid_t pid)
    {
        errno = ENOENT;
        int status;
        if (-1 == waitpid(pid, &status, 0)) {
            const auto message = fmt::format("wait system call failed. [errno: {0}]", errno);
            return rust::Err(std::runtime_error(message));
        } else {
            const int result = WIFEXITED(status) ? WEXITSTATUS(status) : EXIT_FAILURE;
            return rust::Ok(result);
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
        auto environment = rust::Result<const char**>(rust::Ok(const_cast<const char**>(environ)))
                               .and_then<std::map<std::string, std::string>>([](auto ptr) {
                                   auto map = start::to_map(ptr);
                                   return rust::Ok(map);
                               })
                               .and_then<std::map<std::string, std::string>>([this](auto map) {
                                   return update(map);
                               });
        auto program = start::executable_name(command.front());

        return rust::merge(program, environment)
            .and_then<pid_t>([&command, this](auto pair) {
                const auto& [program, environment] = pair;
                // create the argument list
                std::vector<std::string_view> args = {
                    executor_.c_str(),
                    er::flags::DESTINATION,
                    server_address_.c_str(),
                    er::flags::LIBRARY, // TODO: deprecate it
                    library_.c_str(), // TODO: deprecate it
                    er::flags::EXECUTE,
                    program.c_str(),
                    er::flags::COMMAND
                };
                std::copy(command.begin(), command.end(), std::back_insert_iterator(args));
                return start::spawn(executor_, args, environment);
            })
            .and_then<int>([](auto pid) {
                return start::wait(pid);
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
