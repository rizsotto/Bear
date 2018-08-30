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

#pragma once

#include "config.h"

#include "libexec_a/Array.h"
#include "libexec_a/Environment.h"
#include "libexec_a/Interface.h"

namespace {

    size_t length(const ::pear::Execution &execution) noexcept {
        return ((execution.path != nullptr) ? 2 : 0) +
               ((execution.file != nullptr) ? 2 : 0) +
               ((execution.search_path != nullptr) ? 2 : 0) +
               ::ear::array::length(execution.command) +
               2;
    }

    const char **copy(const ::pear::Execution &execution, const char **it, const char **it_end) noexcept {
        if (execution.path != nullptr) {
            *it++ = ::pear::flag::path;
            *it++ = execution.path;
        }
        if (execution.file != nullptr) {
            *it++ = ::pear::flag::file;
            *it++ = execution.file;
        }
        if (execution.search_path != nullptr) {
            *it++ = ::pear::flag::search_path;
            *it++ = execution.search_path;
        }
        *it++ = ::pear::flag::command;
        const size_t command_size = ::ear::array::length(execution.command);
        const char **const command_end = execution.command + (command_size + 1);
        return ::ear::array::copy(execution.command, command_end, it, it_end);
    }
}

namespace ear {

    template<typename Resolver>
    class Executor {
    public:
        int execve(const char *path, char *const argv[], char *const envp[]) const noexcept {
            auto fp = Resolver::resolve_execve();
            if (fp == nullptr)
                return -1;

            const ::pear::Execution execution = { const_cast<const char **>(argv), path, nullptr, nullptr };

            const size_t dst_length = length(execution) + session_size_;
            const char *dst[dst_length];
            const char **const dst_end = dst + dst_length;

            const char **it = ::ear::array::copy(session_begin(), session_end(), dst, dst_end);
            if (copy(execution, it, dst_end) == nullptr)
                return -1;

            return fp(reporter(), const_cast<char *const *>(dst), envp);
        }

        int execvpe(const char *file, char *const argv[], char *const envp[]) const noexcept {
            auto fp = Resolver::resolve_execve();
            if (fp == nullptr)
                return -1;

            const ::pear::Execution execution = { const_cast<const char **>(argv), nullptr, file, nullptr };

            const size_t dst_length = length(execution) + session_size_;
            const char *dst[dst_length];
            const char **const dst_end = dst + dst_length;

            const char **it = ::ear::array::copy(session_begin(), session_end(), dst, dst_end);
            if (copy(execution, it, dst_end) == nullptr)
                return -1;

            return fp(reporter(), const_cast<char *const *>(dst), envp);
        }

        int execvP(const char *file, const char *search_path, char *const argv[], char *const envp[]) const noexcept {
            auto fp = Resolver::resolve_execve();
            if (fp == nullptr)
                return -1;

            const ::pear::Execution execution = { const_cast<const char **>(argv), nullptr, file, search_path };

            const size_t dst_length = length(execution) + session_size_;
            const char *dst[dst_length];
            const char **const dst_end = dst + dst_length;

            const char **it = ::ear::array::copy(session_begin(), session_end(), dst, dst_end);
            if (copy(execution, it, dst_end) == nullptr)
                return -1;

            return fp(reporter(), const_cast<char *const *>(dst), envp);
        }

        int posix_spawn(pid_t *pid, const char *path,
                        const posix_spawn_file_actions_t *file_actions,
                        const posix_spawnattr_t *attrp,
                        char *const argv[],
                        char *const envp[]) const noexcept {
            auto fp = Resolver::resolve_spawn();
            if (fp == nullptr)
                return -1;

            const ::pear::Execution execution = { const_cast<const char **>(argv), path, nullptr, nullptr };

            const size_t dst_length = length(execution) + session_size_;
            const char *dst[dst_length];
            const char **const dst_end = dst + dst_length;

            const char **it = ::ear::array::copy(session_begin(), session_end(), dst, dst_end);
            if (copy(execution, it, dst_end) == nullptr)
                return -1;

            return fp(pid, reporter(), file_actions, attrp, const_cast<char *const *>(dst), envp);
        }

        int posix_spawnp(pid_t *pid, const char *file,
                         const posix_spawn_file_actions_t *file_actions,
                         const posix_spawnattr_t *attrp,
                         char *const argv[],
                         char *const envp[]) const noexcept {
            auto fp = Resolver::resolve_spawn();
            if (fp == nullptr)
                return -1;

            const ::pear::Execution execution = { const_cast<const char **>(argv), nullptr, file, nullptr };

            const size_t dst_length = length(execution) + session_size_;
            const char *dst[dst_length];
            const char **const dst_end = dst + dst_length;

            const char **it = ::ear::array::copy(session_begin(), session_end(), dst, dst_end);
            if (copy(execution, it, dst_end) == nullptr)
                return -1;

            return fp(pid, reporter(), file_actions, attrp, const_cast<char *const *>(dst), envp);
        }

    public:
        explicit Executor(const ::ear::LibrarySession &session) noexcept
                : session_ {
                        session.context.reporter,
                        ::pear::flag::destination,
                        session.context.destination,
                        ::pear::flag::library,
                        session.library,
                        (session.context.verbose) ? ::pear::flag::verbose : nullptr,
                        nullptr }
                , session_size_(::ear::array::length(session_))
        { }

        explicit Executor(const ::ear::WrapperSession &session) noexcept
                : session_ {
                        session.context.reporter,
                        ::pear::flag::destination,
                        session.context.destination,
                        (session.context.verbose) ? ::pear::flag::verbose : nullptr,
                        nullptr }
                , session_size_(::ear::array::length(session_))
        { }

        Executor() noexcept = delete;

        Executor(const Executor &) = delete;

        Executor(Executor &&) noexcept = delete;

        ~Executor() noexcept = default;

        Executor &operator=(const Executor &) = delete;

        Executor &operator=(Executor &&) noexcept = delete;

    private:
        const char *reporter() const noexcept {
            return session_[0];
        }

        const char **session_begin() const noexcept {
            return const_cast<const char **>(session_);
        }

        const char **session_end() const noexcept {
            return session_begin() + session_size_;
        }

    private:
        static constexpr size_t SESSION_SIZE = 8;

        const char *session_[SESSION_SIZE];
        const size_t session_size_;
    };

}