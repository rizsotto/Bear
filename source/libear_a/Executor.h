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

#include "libear_a/Array.h"
#include "libear_a/Environment.h"
#include "libear_a/Interface.h"

namespace ear {

    template<typename Resolver>
    class Executor {
    public:
        int execve(const char *path, char *const argv[], char *const envp[]) const noexcept {
            if (! initialized_)
                return -1;

            auto fp = Resolver::execve();
            if (fp == nullptr)
                return -1;

            const size_t argv_length = ::ear::array::length(argv);
            const size_t dst_length = argv_length + 7;
            const char *dst[dst_length] = {};

            const char **const dst_end = dst + dst_length;
            const char **it = ::ear::array::copy(session_begin(), session_end(), dst, dst_end);
            *it++ = command_flag;

            ::ear::array::copy(argv, argv + argv_length + 1, it, dst_end);

            return fp(reporter(), const_cast<char *const *>(dst), envp);
        }

        int execvpe(const char *file, char *const argv[], char *const envp[]) const noexcept {
            if (! initialized_)
                return -1;

            auto fp = Resolver::execve();
            if (fp == nullptr)
                return -1;

            const size_t argv_length = ::ear::array::length(argv);
            const size_t dst_length = argv_length + 9;
            const char *dst[dst_length] = {};

            const char **const dst_end = dst + dst_length;
            const char **it = ::ear::array::copy(session_begin(), session_end(), dst, dst_end);
            *it++ = file_flag;
            *it++ = file;
            *it++ = command_flag;

            ::ear::array::copy(argv, argv + argv_length + 1, it, dst + dst_length);

            return fp(reporter(), const_cast<char *const *>(dst), envp);
        }

        int execvP(const char *file, const char *search_path, char *const argv[], char *const envp[]) const noexcept {
            if (! initialized_)
                return -1;

            auto fp = Resolver::execve();
            if (fp == nullptr)
                return -1;

            const size_t argv_length = ::ear::array::length(argv);
            const size_t dst_length = argv_length + 11;
            const char *dst[dst_length] = {};

            const char **const dst_end = dst + dst_length;
            const char **it = ::ear::array::copy(session_begin(), session_end(), dst, dst_end);
            *it++ = file_flag;
            *it++ = file;
            *it++ = search_flag;
            *it++ = search_path;
            *it++ = command_flag;

            ::ear::array::copy(argv, argv + argv_length + 1, it, dst + dst_length);

            return fp(reporter(), const_cast<char *const *>(dst), envp);
        }

        int posix_spawn(pid_t *pid, const char *path,
                        const posix_spawn_file_actions_t *file_actions,
                        const posix_spawnattr_t *attrp,
                        char *const argv[],
                        char *const envp[]) const noexcept {
            if (! initialized_)
                return -1;

            auto fp = Resolver::posix_spawn();
            if (fp == nullptr)
                return -1;

            const size_t argv_length = ::ear::array::length(argv);
            const size_t dst_length = argv_length + 7;
            const char *dst[dst_length] = {};

            const char **const dst_end = dst + dst_length;
            const char **it = ::ear::array::copy(session_begin(), session_end(), dst, dst_end);
            *it++ = command_flag;

            ::ear::array::copy(argv, argv + argv_length + 1, it, dst + dst_length);

            return fp(pid, reporter(), file_actions, attrp, const_cast<char *const *>(dst), envp);
        }

        int posix_spawnp(pid_t *pid, const char *file,
                         const posix_spawn_file_actions_t *file_actions,
                         const posix_spawnattr_t *attrp,
                         char *const argv[],
                         char *const envp[]) const noexcept {
            if (! initialized_)
                return -1;

            auto fp = Resolver::posix_spawn();
            if (fp == nullptr)
                return -1;

            const size_t argv_length = ::ear::array::length(argv);
            const size_t dst_length = argv_length + 9;
            const char *dst[dst_length] = {};

            const char **const dst_end = dst + dst_length;
            const char **it = ::ear::array::copy(session_begin(), session_end(), dst, dst_end);
            *it++ = file_flag;
            *it++ = file;
            *it++ = command_flag;

            ::ear::array::copy(argv, argv + argv_length + 1, it, dst + dst_length);

            return fp(pid, reporter(), file_actions, attrp, const_cast<char *const *>(dst), envp);
        }

    public:
        explicit Executor(const ::ear::LibrarySession *session) noexcept
                : session_ {
            (session != nullptr) ? session->session.reporter : nullptr,
            (session != nullptr) ? destination_flag : nullptr,
            (session != nullptr) ? session->session.destination : nullptr,
            (session != nullptr) ? library_flag : nullptr,
            (session != nullptr) ? session->library : nullptr }
                , initialized_(session != nullptr)
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
            return const_cast<const char **>(session_ + 5);
        }

    private:
        const char *session_[5];
        bool initialized_;
    };

}