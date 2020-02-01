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

#include <cstdio>
#include "Session.h"

#include "libexec.h"
#include "Environment.h"
#include "Storage.h"


namespace ear {

    Session Session::from(const char **environment) noexcept {
        if (nullptr == environment)
            return {};
        else
            return {
                    environment::get_env_value(environment, env::KEY_LIBRARY),
                    environment::get_env_value(environment, env::KEY_REPORTER),
                    environment::get_env_value(environment, env::KEY_DESTINATION),
                    environment::get_env_value(environment, env::KEY_VERBOSE) != nullptr
            };
    }

    void Session::persist(Storage &storage) noexcept {
        if (is_not_valid())
            return;

        library_ = storage.store(library_);
        reporter_ = storage.store(reporter_);
        destination_ = storage.store(destination_);
    }

    void Session::write_message(const char *message) const noexcept {
        if (is_verbose())
            fprintf(stderr, "libexec.so: %s\n", message);
    }
}
