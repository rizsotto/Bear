/*  Copyright (C) 2012-2018 by László Nagy
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

#include "Executor.h"

#include "intercept.h"

#include "Array.h"
#include "Resolver.h"
#include "Session.h"

namespace {

    struct Execution {
        const char **command;
        const char *path;
        const char *file;
        const char *search_path;
    };

    size_t length(Execution const &execution) noexcept {
        return ((execution.path != nullptr) ? 2 : 0) +
               ((execution.file != nullptr) ? 2 : 0) +
               ((execution.search_path != nullptr) ? 2 : 0) +
               ear::array::length(execution.command) +
               2;
    }

    const char **copy(Execution const &execution, const char **it, const char **it_end) noexcept {
        if (execution.path != nullptr) {
            *it++ = pear::flag::PATH;
            *it++ = execution.path;
        }
        if (execution.file != nullptr) {
            *it++ = pear::flag::FILE;
            *it++ = execution.file;
        }
        if (execution.search_path != nullptr) {
            *it++ = pear::flag::SEARCH_PATH;
            *it++ = execution.search_path;
        }
        *it++ = pear::flag::COMMAND;
        const size_t command_size = ear::array::length(execution.command);
        const char **const command_end = execution.command + (command_size + 1);
        return ear::array::copy(execution.command, command_end, it, it_end);
    }

    size_t length(ear::Session const &session) noexcept {
        return session.is_not_valid() ? 5 : 6;
    }

    const char **copy(ear::Session const &session, const char **it, const char **it_end) noexcept {
        *it++ = session.get_reporter();
        *it++ = pear::flag::DESTINATION;
        *it++ = session.get_destination();
        *it++ = pear::flag::LIBRARY;
        *it++ = session.get_library();
        if (session.is_verbose())
            *it++ = pear::flag::VERBOSE;
        return it;
    }

#define CHECK_SESSION(SESSION_) \
    do { \
        if (SESSION_.is_not_valid()) { \
            SESSION_.write_message("not initialized."); \
            return -1; \
        } \
    } while (false)

#define CHECK_FP(SESSION_, FP_) \
    do { \
        if (FP_ == nullptr) { \
            SESSION_.write_message("could not resolve symbol."); \
            return -1; \
        } \
    } while (false)

#define CREATE_BUFFER(VAR_, SESSION_, EXECUTION_) \
    const size_t VAR_##_length = length(EXECUTION_) + length(SESSION_); \
    const char *VAR_[VAR_##_length]; \
    { \
        const char **const VAR_##_end = VAR_ + VAR_##_length; \
        const char **VAR_##it = copy(SESSION_, VAR_, VAR_##_end); \
        copy(EXECUTION_, VAR_##it, VAR_##_end); \
    }

}

namespace ear {

    Executor::Executor(ear::Session const &session, ear::Resolver const &resolver) noexcept
            : session_(session)
            , resolver_(resolver)
    { }

    int Executor::execve(const char *path, char *const *argv, char *const *envp) const noexcept {
        CHECK_SESSION(session_);

        auto fp = resolver_.execve();
        CHECK_FP(session_, fp);

        const Execution execution = { const_cast<const char **>(argv), path, nullptr, nullptr };
        CREATE_BUFFER(dst, session_, execution);

        return fp(session_.get_reporter(), const_cast<char *const *>(dst), envp);
    }

    int Executor::execvpe(const char *file, char *const *argv, char *const *envp) const noexcept {
        CHECK_SESSION(session_);

        auto fp = resolver_.execve();
        CHECK_FP(session_, fp);

        const Execution execution = { const_cast<const char **>(argv), nullptr, file, nullptr };
        CREATE_BUFFER(dst, session_, execution);

        return fp(session_.get_reporter(), const_cast<char *const *>(dst), envp);
    }

    int Executor::execvP(const char *file, const char *search_path, char *const *argv,
                         char *const *envp) const noexcept {
        CHECK_SESSION(session_);

        auto fp = resolver_.execve();
        CHECK_FP(session_, fp);

        const Execution execution = { const_cast<const char **>(argv), nullptr, file, search_path };
        CREATE_BUFFER(dst, session_, execution);

        return fp(session_.get_reporter(), const_cast<char *const *>(dst), envp);
    }

    int Executor::posix_spawn(pid_t *pid, const char *path, const posix_spawn_file_actions_t *file_actions,
                              const posix_spawnattr_t *attrp, char *const *argv,
                              char *const *envp) const noexcept {
        CHECK_SESSION(session_);

        auto fp = resolver_.posix_spawn();
        CHECK_FP(session_, fp);

        const Execution execution = { const_cast<const char **>(argv), path, nullptr, nullptr };
        CREATE_BUFFER(dst, session_, execution);

        return fp(pid, session_.get_reporter(), file_actions, attrp, const_cast<char *const *>(dst), envp);
    }

    int Executor::posix_spawnp(pid_t *pid, const char *file, const posix_spawn_file_actions_t *file_actions,
                               const posix_spawnattr_t *attrp, char *const *argv,
                               char *const *envp) const noexcept {
        CHECK_SESSION(session_);

        auto fp = resolver_.posix_spawn();
        CHECK_FP(session_, fp);

        const Execution execution = { const_cast<const char **>(argv), nullptr, file, nullptr };
        CREATE_BUFFER(dst, session_, execution);

        return fp(pid, session_.get_reporter(), file_actions, attrp, const_cast<char *const *>(dst), envp);
    }
}
