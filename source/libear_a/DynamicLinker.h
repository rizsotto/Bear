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

#include <dlfcn.h>
#if defined HAVE_SPAWN_HEADER
# include <spawn.h>
#endif

#include "libear_a/Result.h"
#include "libear_a/Resolver.h"

namespace ear {

    template <typename F>
    F typed_dlsym(const char *const name) {
        void *symbol = dlsym(RTLD_NEXT, name);
        return reinterpret_cast<F>(symbol);
    }

    struct DynamicLinker {

        using execve_t = int (*)(const char *path, char *const argv[], char *const envp[]);
        static execve_t execve() noexcept {
            return typed_dlsym<execve_t>("execve");
        }

        using execv_t = int (*)(const char *path, char *const argv[]);
        static execv_t execv() noexcept {
            return typed_dlsym<execv_t>("execv");
        }

        using execvpe_t = int (*)(const char *file, char *const argv[], char *const envp[]);
        static execve_t execvpe() noexcept {
            return typed_dlsym<execvpe_t>("execvpe");
        }

        using execvp_t = int (*)(const char *file, char *const argv[]);
        static execvp_t execvp() noexcept {
            return typed_dlsym<execvp_t>("execvp");
        }

        using execvP_t = int (*)(const char *file, const char *search_path, char *const argv[]);
        static execvP_t execvP() noexcept {
            return typed_dlsym<execvP_t>("execvP");
        }

        using exect_t = int (*)(const char *path, char *const argv[], char *const envp[]);
        static exect_t exect() noexcept {
            return typed_dlsym<exect_t>("exect");
        }

        using posix_spawn_t = int (*)(pid_t *pid,
                                      const char *path,
                                      const posix_spawn_file_actions_t *file_actions,
                                      const posix_spawnattr_t *attrp,
                                      char *const argv[],
                                      char *const envp[]);
        static posix_spawn_t posix_spawn() noexcept {
            return typed_dlsym<posix_spawn_t>("posix_spawn");
        }

        using posix_spawnp_t = int (*)(pid_t *pid,
                                       const char *file,
                                       const posix_spawn_file_actions_t *file_actions,
                                       const posix_spawnattr_t *attrp,
                                       char *const argv[],
                                       char *const envp[]);
        static posix_spawnp_t posix_spawnp() noexcept {
            return typed_dlsym<posix_spawnp_t>("posix_spawnp");
        }
    };

    struct DynamicLinker_Z : public Resolver {
        Result<Execve> execve() const noexcept override {
            constexpr char execve_name[] = "execve";
            return typed_dlsym_Z<Execve_Fp>(execve_name)
                    .map<Execve>([](const Execve_Fp &fp) { return Execve(fp); });
        }

        Result<Execve> execvpe() const noexcept override {
            constexpr char execvpe_name[] = "execvpe";
            return typed_dlsym_Z<Execve_Fp>(execvpe_name)
                    .map<Execve>([](const Execve_Fp &fp) { return Execve(fp); });
        }

        Result<ExecvP> execvP() const noexcept override {
            constexpr char execvp_name[] = "execvP";
            return typed_dlsym_Z<ExecvP_Fp>(execvp_name)
                    .map<ExecvP>([](const ExecvP_Fp &fp) { return ExecvP(fp); });
        }

        Result<Spawn> posix_spawn() const noexcept override {
            constexpr char posix_spawn_name[] = "posix_spawn";
            return typed_dlsym_Z<Spawn_Fp>(posix_spawn_name)
                    .map<Spawn>([](const Spawn_Fp &fp) { return Spawn(fp); });
        }

        Result<Spawn> posix_spawnp() const noexcept override {
            constexpr char posix_spawnp_name[] = "posix_spawnp";
            return typed_dlsym_Z<Spawn_Fp>(posix_spawnp_name)
                    .map<Spawn>([](const Spawn_Fp &fp) { return Spawn(fp); });
        }

    private:
        template <typename F>
        static Result<F> typed_dlsym_Z(const char *name) {
            // TODO: create a new exception type to store the symbol name
            void *symbol = dlsym(RTLD_NEXT, name);
            if (symbol == nullptr)
                return Result<F>::failure(std::runtime_error("Couldn't resolve symbol"));

            auto result = reinterpret_cast<F>(symbol);
            if (result == nullptr)
                return Result<F>::failure(std::runtime_error("Couldn't cast symbol"));

            return Result<F>::success(result);
        }

    };

}
