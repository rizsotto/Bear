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

#include "Application.h"
#include "Session.h"
#include "libsys/Path.h"
#include "libsys/Process.h"
#ifdef SUPPORT_PRELOAD
#include "er/Flags.h"
#include "libexec/Environment.h"
#endif

#include <functional>
#include <numeric>

#include <spdlog/fmt/ostr.h>
#include <spdlog/spdlog.h>

namespace env {

    constexpr char GLIBC_PRELOAD_KEY[] = "LD_PRELOAD";

    using env_t = std::map<std::string, std::string>;
    using mapper_t = std::function<std::string(const std::string&, const std::string&)>;

    std::string
    merge_into_paths(const std::string& current, const std::string& value) noexcept
    {
        auto paths = sys::path::split(current);
        if (std::find(paths.begin(), paths.end(), value) == paths.end()) {
            paths.emplace_front(value);
            return sys::path::join(paths);
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

template <typename T>
std::ostream& operator<<(std::ostream& os, const std::vector<T>& v)
{
    std::copy(v.begin(), v.end(), std::ostream_iterator<T>(os, " "));
    return os;
}

#ifdef SUPPORT_PRELOAD
namespace {

    class LibraryPreloadSession : public ic::Session {
    public:
        LibraryPreloadSession(
            const std::string_view& library,
            const std::string_view& executor,
            const bool verbose,
            env::env_t&& environment);

    public:
        [[nodiscard]] rust::Result<std::string_view> resolve(const std::string& name) const override;
        [[nodiscard]] rust::Result<std::map<std::string, std::string>> update(const std::map<std::string, std::string>& env) const override;
        [[nodiscard]] rust::Result<sys::Process::Builder> supervise(const std::vector<std::string_view>& command) const override;

        void set_server_address(const std::string&) override;

        [[nodiscard]] std::string get_session_type() const override;

    private:
        std::string server_address_;
        std::string library_;
        std::string executor_;
        bool verbose_;
        env::env_t environment_;
    };

    LibraryPreloadSession::LibraryPreloadSession(
        const std::string_view& library,
        const std::string_view& executor,
        bool verbose,
        env::env_t&& environment)
            : server_address_()
            , library_(library)
            , executor_(executor)
            , verbose_(verbose)
            , environment_(environment)
    {
        spdlog::debug("Created library preload session. [library={0}, executor={1}]", library_, executor_);
    }

    rust::Result<std::string_view> LibraryPreloadSession::resolve(const std::string& name) const
    {
        return rust::Err(std::runtime_error("The session does not support resolve."));
    }

    rust::Result<std::map<std::string, std::string>> LibraryPreloadSession::update(const std::map<std::string, std::string>& env) const
    {
        std::map<std::string, std::string> copy(env);
        if (verbose_) {
            env::insert_or_assign(copy, el::env::KEY_VERBOSE, "true");
        }
        env::insert_or_assign(copy, el::env::KEY_REPORTER, executor_);
        env::insert_or_assign(copy, el::env::KEY_DESTINATION, server_address_);
        env::insert_or_merge(copy, env::GLIBC_PRELOAD_KEY, library_, env::merge_into_paths);

        return rust::Ok(copy);
    }

    rust::Result<sys::Process::Builder> LibraryPreloadSession::supervise(const std::vector<std::string_view>& command) const
    {
        auto environment = update(environment_);
        auto program = sys::Process::Builder(command.front()).resolve_executable();

        return rust::merge(program, environment)
            .map<sys::Process::Builder>([&command, this](auto pair) {
                const auto& [program, environment] = pair;
                return sys::Process::Builder(executor_)
                    .add_argument(executor_)
                    .add_argument(er::flags::DESTINATION)
                    .add_argument(server_address_)
                    .add_argument(er::flags::EXECUTE)
                    .add_argument(program)
                    .add_argument(er::flags::COMMAND)
                    .add_arguments(command.begin(), command.end())
                    .set_environment(environment);
            });
    }

    void LibraryPreloadSession::set_server_address(const std::string& value)
    {
        server_address_ = value;
    }

    std::string LibraryPreloadSession::get_session_type() const
    {
        return std::string("library preload");
    }
}
#endif

namespace ic {

#ifdef SUPPORT_PRELOAD
    rust::Result<Session::SharedPtr> Session::from(const flags::Arguments& args, const sys::Context& ctx)
    {
        auto library = args.as_string(ic::Application::LIBRARY);
        auto executor = args.as_string(ic::Application::EXECUTOR);
        auto verbose = args.as_bool(ic::Application::VERBOSE);

        return merge(library, executor, verbose)
            .map<Session::SharedPtr>([&ctx](auto tuple) {
                const auto& [library, executor, verbose] = tuple;
                auto environment = ctx.get_environment();
                auto result = new LibraryPreloadSession(library, executor, verbose, std::move(environment));
                return std::shared_ptr<Session>(result);
            });
    }
#else
    rust::Result<Session::SharedPtr> Session::from(const flags::Arguments& args, const sys::Context& ctx)
    {
        return rust::Err(std::runtime_error("Not implemented."));
    }
#endif
}
