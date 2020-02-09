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

#include "Session.h"

#include "Buffer.h"
#include "Environment.h"
#include "libexec.h"

namespace ear {

    namespace session {

        void from(Session& session, const char** environment) noexcept
        {
            if (nullptr == environment)
                return;

            session.library = env::get_env_value(environment, env::KEY_LIBRARY);
            session.reporter = env::get_env_value(environment, env::KEY_REPORTER);
            session.destination = env::get_env_value(environment, env::KEY_DESTINATION);
            session.verbose = env::get_env_value(environment, env::KEY_VERBOSE) != nullptr;
        }

        void persist(Session& session, char* begin, char* end) noexcept
        {
            if (!is_valid(session))
                return;

            Buffer buffer(begin, end);
            session.library = buffer.store(session.library);
            session.reporter = buffer.store(session.reporter);
            session.destination = buffer.store(session.destination);
        }

        bool is_valid(Session const& session) noexcept
        {
            return (session.library != nullptr && session.reporter != nullptr && session.destination != nullptr);
        }
    }
}
