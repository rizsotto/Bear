/*  Copyright (C) 2012-2017 by László Nagy
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

#include "libear_a/Resolver.h"
#include "libear_a/Execution.h"
#include "libear_a/Array.h"


namespace {

    /**
     * Simple call forwarding method which bind all parameters.
     *
     * @tparam E execution type
     * @param resolver the symbol resolver
     * @param execution the execution parameters
     * @return function to execute.
     */
    template<typename E>
    ear::Result<ear::Resolver::Execution>
    resolve_(ear::Resolver const &resolver, E const &execution) noexcept;

    template<>
    ear::Result<ear::Resolver::Execution>
    resolve_(ear::Resolver const &resolver, ear::Execve_Z const &execution) noexcept {
        return resolver.execve()
                .map<ear::Resolver::Execution>([&execution](auto fp) {
                    return std::bind(fp,
                                     execution.path_,
                                     const_cast<char *const *>(execution.argv_),
                                     const_cast<char *const *>(execution.envp_));
                });
    }

    template<>
    ear::Result<ear::Resolver::Execution>
    resolve_(ear::Resolver const &resolver, ear::Execvpe_Z const &execution) noexcept {
        return resolver.execvpe()
                .map<ear::Resolver::Execution>([&execution](auto fp) {
                    return std::bind(fp,
                                     execution.file_,
                                     const_cast<char *const *>(execution.argv_),
                                     const_cast<char *const *>(execution.envp_));
                });
    }

    template<>
    ear::Result<ear::Resolver::Execution>
    resolve_(ear::Resolver const &resolver, ear::ExecvP_Z const &execution) noexcept {
        return resolver.execvP()
                .map<ear::Resolver::Execution>([&execution](auto fp) {
                    return std::bind(fp,
                                     execution.file_,
                                     execution.search_path_,
                                     const_cast<char *const *>(execution.argv_));
                });
    }

    template<>
    ear::Result<ear::Resolver::Execution>
    resolve_(ear::Resolver const &resolver, ear::Spawn_Z const &execution) noexcept {
        return resolver.posix_spawn()
                .map<ear::Resolver::Execution>([&execution](auto fp) {
                    return std::bind(fp,
                                     execution.pid_,
                                     execution.path_,
                                     execution.file_actions_,
                                     execution.attrp_,
                                     const_cast<char *const *>(execution.argv_),
                                     const_cast<char *const *>(execution.envp_));
                });
    }

    template<>
    ear::Result<ear::Resolver::Execution>
    resolve_(ear::Resolver const &resolver, ear::Spawnp_Z const &execution) noexcept {
        return resolver.posix_spawn()
                .map<ear::Resolver::Execution>([&execution](auto fp) {
                    return std::bind(fp,
                                     execution.pid_,
                                     execution.file_,
                                     execution.file_actions_,
                                     execution.attrp_,
                                     const_cast<char *const *>(execution.argv_),
                                     const_cast<char *const *>(execution.envp_));
                });
    }



    using PartialExecution = std::function<int (const char **)>;

    ear::Result<PartialExecution>
    bind_execve_(ear::Resolver const &resolver, ear::Execution_Z const &execution) noexcept {
        return resolver.execve()
                .map<PartialExecution>([&execution](auto fp) {
                    return [&execution, &fp](const char **args) {
                        return fp(args[0],
                                  const_cast<char *const *>(args),
                                  const_cast<char *const *>(execution.envp_));
                    };
                });
    }

    ear::Result<PartialExecution>
    bind_spawn(ear::Resolver const &resolver, ear::ExecutionWithoutFork_Z const &execution) noexcept {
        return resolver.posix_spawn()
                .map<PartialExecution>([&execution](auto fp) {
                    return [&execution, &fp](const char **args) {
                        return fp(execution.pid_,
                                  args[0],
                                  execution.file_actions_,
                                  execution.attrp_,
                                  const_cast<char *const *>(args),
                                  const_cast<char *const *>(execution.envp_));
                    };
                });
    }

    using Estimator = std::function<size_t ()>;
    using Copier = std::function<const char ** (const char **, const char **)>;
    using BufferOps = std::pair<Estimator, Copier>;

    ear::Resolver::Execution
    bind_(PartialExecution const &fp, BufferOps const &session, BufferOps const &execution) noexcept {
        const size_t session_size = session.first();
        const size_t execution_size = execution.first();
        const size_t size = session_size + execution_size + 1;
        const char *buffer[size];
        const char **it = buffer;
        const char **const end = it + size;

        it = session.second(it, end);
        it = execution.second(it, end);
        *it++ = nullptr;

        return std::bind(fp, const_cast<const char **>(buffer));
    }


    template<typename E>
    ear::Result<ear::Resolver::Execution>
    resolve_(ear::Resolver const &resolver, BufferOps const &session, E const &execution) noexcept;

    template<>
    ear::Result<ear::Resolver::Execution>
    resolve_(ear::Resolver const &resolver, BufferOps const &session, ear::Execve_Z const &execution) noexcept {
        auto execution_functions = std::make_pair(
                [&execution]() { return ear::array::length(execution.argv_) + 1; },
                [&execution](const char **begin, const char **end) {
                    *begin++ = ear::command_separator;
                    return ear::array::copy(execution.argv_,
                                            execution.argv_ + ear::array::length(execution.argv_),
                                            begin,
                                            end);
                });

        return bind_execve_(resolver, execution)
                .map<ear::Resolver::Execution>([&session, &execution_functions](auto fp) {
                    return bind_(fp, session, execution_functions);
                });
    }

    template<typename S, typename E>
    ear::Result<ear::Resolver::Execution>
    resolve_(ear::Resolver const &resolver, S const &session, E const &execution) noexcept;

    template<typename E>
    ear::Result<ear::Resolver::Execution>
    resolve_(ear::Resolver const &resolver, ear::LibrarySession const &session, E const &execution) noexcept {
        auto session_functions = std::make_pair(
                [&session]() {
                    return (session.verbose) ? 8 : 7;
                },
                [&session](const char **it, const char **end) {
                    *it++ = session.reporter;
                    *it++ = "--report";
                    *it++ = ear::destination_flag;
                    *it++ = session.destination;
                    *it++ = ear::library_flag;
                    *it++ = session.library;
                    if (session.verbose)
                        *it++ = ear::verbose_flag;

                    return it;
                });

        return resolve_(resolver, session_functions, execution);
    }

}

namespace ear {

    template<typename S, typename E>
    Result<ear::Resolver::Execution> Resolver::resolve(const S *session, const E &execution) const noexcept {
        return (session == nullptr)
                ? resolve_(*this, execution)
                : resolve_(*this, *session, execution);
    }

}