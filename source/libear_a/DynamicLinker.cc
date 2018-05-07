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

#include "libear_a/DynamicLinker.h"

namespace {

    using Execve_Fp = int (*)(const char *, char *const *, char *const *);
    using ExecvP_Fp = int (*)(const char *, const char *, char *const *);
    using Spawn_Fp  = int (*)(pid_t *,
                              const char *,
                              const posix_spawn_file_actions_t *,
                              const posix_spawnattr_t *,
                              char *const *,
                              char *const *);


    template<typename F>
    ear::Result<F> typed_dlsym_Z(const char *name) {
        // TODO: create a new exception type to store the symbol name
        void *symbol = dlsym(RTLD_NEXT, name);
        if (symbol == nullptr)
            return ear::Result<F>::failure(std::runtime_error("Couldn't resolve symbol"));

        auto result = reinterpret_cast<F>(symbol);
        if (result == nullptr)
            return ear::Result<F>::failure(std::runtime_error("Couldn't cast symbol"));

        return ear::Result<F>::success(result);
    }

}

namespace ear {

    Result<Resolver::Execve> DynamicLinker_Z::execve() const noexcept {
        constexpr char execve_name[] = "execve";
        return typed_dlsym_Z<Execve_Fp>(execve_name)
                .map<Execve>([](const Execve_Fp &fp) { return Execve(fp); });
    }

    Result<Resolver::Execve> DynamicLinker_Z::execvpe() const noexcept {
        constexpr char execvpe_name[] = "execvpe";
        return typed_dlsym_Z<Execve_Fp>(execvpe_name)
                .map<Execve>([](const Execve_Fp &fp) { return Execve(fp); });
    }

    Result<Resolver::ExecvP> DynamicLinker_Z::execvP() const noexcept {
        constexpr char execvp_name[] = "execvP";
        return typed_dlsym_Z<ExecvP_Fp>(execvp_name)
                .map<ExecvP>([](const ExecvP_Fp &fp) { return ExecvP(fp); });
    }

    Result<Resolver::Spawn> DynamicLinker_Z::posix_spawn() const noexcept {
        constexpr char posix_spawn_name[] = "posix_spawn";
        return typed_dlsym_Z<Spawn_Fp>(posix_spawn_name)
                .map<Spawn>([](const Spawn_Fp &fp) { return Spawn(fp); });
    }

    Result<Resolver::Spawn> DynamicLinker_Z::posix_spawnp() const noexcept {
        constexpr char posix_spawnp_name[] = "posix_spawnp";
        return typed_dlsym_Z<Spawn_Fp>(posix_spawnp_name)
                .map<Spawn>([](const Spawn_Fp &fp) { return Spawn(fp); });
    }

}