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

#pragma once

#include "libexec_a/Storage.h"

namespace ear {

    struct Session {
        bool is_not_valid() const noexcept {
            return (library == nullptr || reporter == nullptr || destination == nullptr);
        }

        void persist(Storage &storage) noexcept {
            if (is_not_valid())
                return;

            library = storage.store(library);
            reporter = storage.store(reporter);
            destination = storage.store(destination);
        }

        char const *library;
        char const *reporter;
        char const *destination;
        bool verbose;
    };
}
