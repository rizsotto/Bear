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

#include "collect/SessionLibrary.h"

#include "intercept/Flags.h"
#include "libsys/Errors.h"
#include "libsys/Path.h"
#include "libsys/Process.h"
#include "report/libexec/Environments.h"
#include "report/libexec/Resolver.h"
#include "report/supervisor/Flags.h"

#include <spdlog/spdlog.h>

#include <functional>
#include <utility>

namespace {

    constexpr char GLIBC_PRELOAD_KEY[] = "LD_PRELOAD";

    using env_t = std::map<std::string, std::string>;
    using mapper_t = std::function<std::string(const std::string&, const std::string&)>;

    void insert_or_merge(
        env_t& target,
        const char* key,
        const std::string& value,
        const mapper_t& merger) noexcept
    {
        if (auto it = target.find(key); it != target.end()) {
            it->second = merger(value, it->second);
        } else {
            target.emplace(key, value);
        }
    }
}

namespace ic {

    rust::Result<Session::SharedPtr> LibraryPreloadSession::from(const flags::Arguments& args, const char **envp)
    {
        auto verbose = args.as_bool(flags::VERBOSE).unwrap_or(false);
        auto library = args.as_string(ic::LIBRARY);
        auto executor = args.as_string(ic::EXECUTOR);
        auto environment = sys::env::from(envp);
        auto path = sys::os::get_path(environment);

        return merge(library, executor, path)
            .map<Session::SharedPtr>([&verbose, &environment](auto tuple) {
                const auto& [library, executor, path] = tuple;
                return std::make_shared<LibraryPreloadSession>(library, executor, verbose, path, std::move(environment));
            });
    }

    LibraryPreloadSession::LibraryPreloadSession(
        const std::string_view &library,
        const std::string_view &executor,
        bool verbose,
        const std::string &path,
        sys::env::Vars &&environment)
            : Session()
            , library_(library)
            , executor_(executor)
            , path_(path)
            , verbose_(verbose)
            , environment_(environment)
    {
        spdlog::debug("Created library preload session. [library={0}, executor={1}]", library_, executor_);
    }

    rust::Result<std::string> LibraryPreloadSession::resolve(const std::string&) const
    {
        return rust::Err(std::runtime_error("The session does not support resolve."));
    }

    rust::Result<std::map<std::string, std::string>> LibraryPreloadSession::update(const std::map<std::string, std::string>& env) const
    {
        std::map<std::string, std::string> copy(env);
        if (verbose_) {
            copy[el::env::KEY_VERBOSE] = "true";
        }
        copy[el::env::KEY_DESTINATION] = server_address_;
        copy[el::env::KEY_REPORTER] = executor_;
        insert_or_merge(copy, GLIBC_PRELOAD_KEY, library_, Session::keep_front_in_path);

        return rust::Ok(copy);
    }

    rust::Result<sys::Process::Builder> LibraryPreloadSession::supervise(const std::vector<std::string_view>& command) const
    {
        auto resolver = el::Resolver();
        auto program = resolver.from_search_path(command.front(), path_.c_str())
                .map<std::string>([](auto ptr) {
                    return std::string(ptr);
                })
                .map_err<std::runtime_error>([&command](auto error) {
                    return std::runtime_error(
                            fmt::format("Could not found: {}: {}", command.front(), sys::error_string(error)));
                });
        auto environment = update(environment_);

        return rust::merge(program, environment)
            .map<sys::Process::Builder>([&command, this](auto pair) {
                const auto& [program, environment] = pair;
                auto result = sys::Process::Builder(executor_)
                    .add_argument(executor_)
                    .add_argument(er::DESTINATION)
                    .add_argument(server_address_);
                if (verbose_) {
                    result.add_argument(er::VERBOSE);
                }

                return result
                    .add_argument(er::EXECUTE)
                    .add_argument(program)
                    .add_argument(er::COMMAND)
                    .add_arguments(command.begin(), command.end())
                    .set_environment(environment);
            });
    }

    std::string LibraryPreloadSession::get_session_type() const
    {
        return std::string("library preload");
    }
}
