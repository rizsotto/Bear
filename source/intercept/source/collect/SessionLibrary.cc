/*  Copyright (C) 2012-2023 by László Nagy
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
#include "collect/SessionLibrary.h"

#include "libsys/Errors.h"
#include "libsys/Path.h"
#include "libsys/Process.h"

#include <spdlog/spdlog.h>

#include <functional>

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

    rust::Result<Session::Ptr> LibraryPreloadSession::from(const flags::Arguments& args)
    {
        auto verbose = args.as_bool(flags::VERBOSE).unwrap_or(false);
        const auto library = args.as_string(cmd::intercept::FLAG_LIBRARY);
        const auto wrapper = args.as_string(cmd::intercept::FLAG_WRAPPER);

        return merge(library, wrapper)
            .map<Session::Ptr>([&verbose](auto tuple) {
                const auto& [library, wrapper] = tuple;
                return std::make_shared<LibraryPreloadSession>(verbose, library, wrapper);
            });
    }

    LibraryPreloadSession::LibraryPreloadSession(
        bool verbose,
        const std::string_view &library,
        const std::string_view &executor)
            : Session()
            , verbose_(verbose)
            , library_(library)
            , executor_(executor)
    {
        spdlog::debug("Created library preload session. [library={0}, executor={1}]", library_, executor_);
    }

    rust::Result<ic::Execution> LibraryPreloadSession::resolve(const ic::Execution &execution) const
    {
        spdlog::debug("trying to resolve for library: {}", execution.executable.string());
        return rust::Ok(ic::Execution{
                execution.executable,
                execution.arguments,
                execution.working_dir,
                update(execution.environment)
        });
    }

    sys::Process::Builder LibraryPreloadSession::supervise(const ic::Execution &execution) const
    {
        auto builder = sys::Process::Builder(executor_)
                .add_argument(executor_)
                .add_argument(cmd::wrapper::FLAG_DESTINATION)
                .add_argument(*session_locator_);

        if (verbose_) {
            builder.add_argument(cmd::wrapper::FLAG_VERBOSE);
        }

        return builder
                .add_argument(cmd::wrapper::FLAG_EXECUTE)
                .add_argument(execution.executable)
                .add_argument(cmd::wrapper::FLAG_COMMAND)
                .add_arguments(execution.arguments.begin(), execution.arguments.end())
                .set_environment(update(execution.environment));
    }

    std::map<std::string, std::string>
    LibraryPreloadSession::update(const std::map<std::string, std::string> &env) const
    {
        std::map<std::string, std::string> copy(env);
        if (verbose_) {
            copy[cmd::library::KEY_VERBOSE] = "true";
        }
        copy[cmd::library::KEY_DESTINATION] = *session_locator_;
        copy[cmd::library::KEY_REPORTER] = executor_;
        insert_or_merge(copy, GLIBC_PRELOAD_KEY, library_, Session::keep_front_in_path);

        return copy;
    }
}
