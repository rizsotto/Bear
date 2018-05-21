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
        explicit Executor(const ::ear::LibrarySession *session) noexcept
                : session_(session) {}

        int execve(const char *path, char *const argv[], char *const envp[]) const noexcept {
            if (session_ == nullptr)
                return -1;

            auto fp = Resolver::execve();
            if (fp == nullptr)
                return -1;

            const size_t argv_length = ::ear::array::length(argv);
            const size_t dst_length = argv_length + 7;
            const char *dst[dst_length] = {};

            const char **it = dst;
            *it++ = session_->session.reporter;
            *it++ = destination_flag;
            *it++ = session_->session.destination;
            *it++ = library_flag;
            *it++ = session_->library;
            *it++ = command_flag;

            ::ear::array::copy(argv, argv + argv_length, it, dst + dst_length);

            return fp(session_->session.reporter, const_cast<char *const *>(dst), envp);
        }

        int execvpe(const char *file, char *const argv[], char *const envp[]) const noexcept {
            if (session_ == nullptr)
                return -1;

            auto fp = Resolver::execve();
            if (fp == nullptr)
                return -1;

            const size_t argv_length = ::ear::array::length(argv);
            const size_t dst_length = argv_length + 9;
            const char *dst[dst_length] = {};

            const char **it = dst;
            *it++ = session_->session.reporter;
            *it++ = destination_flag;
            *it++ = session_->session.destination;
            *it++ = library_flag;
            *it++ = session_->library;
            *it++ = file_flag;
            *it++ = file;
            *it++ = command_flag;

            ::ear::array::copy(argv, argv + argv_length, it, dst + dst_length);

            return fp(session_->session.reporter, const_cast<char *const *>(dst), envp);
        }

        int execvP(const char *file, const char *search_path, char *const argv[], char *const envp[]) const noexcept {
            if (session_ == nullptr)
                return -1;

            auto fp = Resolver::execve();
            if (fp == nullptr)
                return -1;

            const size_t argv_length = ::ear::array::length(argv);
            const size_t dst_length = argv_length + 11;
            const char *dst[dst_length] = {};

            const char **it = dst;
            *it++ = session_->session.reporter;
            *it++ = destination_flag;
            *it++ = session_->session.destination;
            *it++ = library_flag;
            *it++ = session_->library;
            *it++ = file_flag;
            *it++ = file;
            *it++ = search_flag;
            *it++ = search_path;
            *it++ = command_flag;

            ::ear::array::copy(argv, argv + argv_length, it, dst + dst_length);

            return fp(session_->session.reporter, const_cast<char *const *>(dst), envp);
        }

        int posix_spawn(pid_t *pid, const char *path,
                        const posix_spawn_file_actions_t *file_actions,
                        const posix_spawnattr_t *attrp,
                        char *const argv[],
                        char *const envp[]) const noexcept {
            if (session_ == nullptr)
                return -1;

            auto fp = Resolver::posix_spawn();
            if (fp == nullptr)
                return -1;

            const size_t argv_length = ::ear::array::length(argv);
            const size_t dst_length = argv_length + 7;
            const char *dst[dst_length] = {};

            const char **it = dst;
            *it++ = session_->session.reporter;
            *it++ = destination_flag;
            *it++ = session_->session.destination;
            *it++ = library_flag;
            *it++ = session_->library;
            *it++ = command_flag;

            ::ear::array::copy(argv, argv + argv_length, it, dst + dst_length);

            return fp(pid, session_->session.reporter, file_actions, attrp,
                      const_cast<char *const *>(dst), envp);
        }

        int posix_spawnp(pid_t *pid, const char *file,
                         const posix_spawn_file_actions_t *file_actions,
                         const posix_spawnattr_t *attrp,
                         char *const argv[],
                         char *const envp[]) const noexcept {
            if (session_ == nullptr)
                return -1;

            auto fp = Resolver::posix_spawn();
            if (fp == nullptr)
                return -1;

            const size_t argv_length = ::ear::array::length(argv);
            const size_t dst_length = argv_length + 9;
            const char *dst[dst_length] = {};

            const char **it = dst;
            *it++ = session_->session.reporter;
            *it++ = destination_flag;
            *it++ = session_->session.destination;
            *it++ = library_flag;
            *it++ = session_->library;
            *it++ = file_flag;
            *it++ = file;
            *it++ = command_flag;

            ::ear::array::copy(argv, argv + argv_length, it, dst + dst_length);

            return fp(pid, session_->session.reporter, file_actions, attrp,
                      const_cast<char *const *>(dst), envp);
        }

    public:
        Executor() noexcept = delete;

        Executor(const Executor &) = delete;

        Executor(Executor &&) noexcept = delete;

        ~Executor() noexcept = default;

        Executor &operator=(const Executor &) = delete;

        Executor &operator=(Executor &&) noexcept = delete;

    private:
        const ::ear::LibrarySession *const session_;
    };

}