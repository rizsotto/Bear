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

#include <dlfcn.h>

namespace {

    template <typename F>
    F typed_dlsym(const char *const name) {
        void *symbol = dlsym(RTLD_NEXT, name);
        return reinterpret_cast<F>(symbol);
    }

}

namespace ear {

    DynamicLinker::execve_t DynamicLinker::execve() noexcept {
        return typed_dlsym<DynamicLinker::execve_t>("execve");
    }

    DynamicLinker::execv_t DynamicLinker::execv() noexcept {
        return typed_dlsym<DynamicLinker::execv_t>("execv");
    }

    DynamicLinker::execve_t DynamicLinker::execvpe() noexcept {
        return typed_dlsym<DynamicLinker::execvpe_t>("execvpe");
    }

    DynamicLinker::execvp_t DynamicLinker::execvp() noexcept {
        return typed_dlsym<DynamicLinker::execvp_t>("execvp");
    }

    DynamicLinker::execvP_t DynamicLinker::execvP() noexcept {
        return typed_dlsym<DynamicLinker::execvP_t>("execvP");
    }

    DynamicLinker::exect_t DynamicLinker::exect() noexcept {
        return typed_dlsym<DynamicLinker::exect_t>("exect");
    }

    DynamicLinker::posix_spawn_t DynamicLinker::posix_spawn() noexcept {
        return typed_dlsym<DynamicLinker::posix_spawn_t>("posix_spawn");
    }

    DynamicLinker::posix_spawnp_t DynamicLinker::posix_spawnp() noexcept {
        return typed_dlsym<DynamicLinker::posix_spawnp_t>("posix_spawnp");
    }

}