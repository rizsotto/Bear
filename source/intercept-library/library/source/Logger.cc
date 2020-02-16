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

#include "Logger.h"

#include "Resolver.h"
#include "Session.h"

#include <cstdio>
#include <unistd.h>

namespace {

    ear::log::Level LEVEL = ear::log::SILENT;

}

namespace ear {
    namespace log {

        void set(Level level)
        {
            LEVEL = level;
        }

        Logger::Logger(const char* name) noexcept
                : name_(name)
        {
        }

        void Logger::debug(char const* message) const noexcept
        {
            if (ear::log::VERBOSE == LEVEL) {
                auto pid = getpid();
                dprintf(STDERR_FILENO, "libexec.so: [pid: %d] %s; %s\n", pid, name_, message);
            }
        }

        void Logger::debug(char const* message, char const* variable) const noexcept
        {
            if (ear::log::VERBOSE == LEVEL) {
                auto pid = getpid();
                dprintf(STDERR_FILENO, "libexec.so: [pid: %d] %s; %s%s\n", pid, name_, message, variable);
            }
        }

        void Logger::warning(char const* message) const noexcept
        {
            auto pid = getpid();
            dprintf(STDERR_FILENO, "libexec.so: [pid: %d] %s; %s\n", pid, name_, message);
        }

        void Logger::warning(char const* message, char const* variable) const noexcept
        {
            auto pid = getpid();
            dprintf(STDERR_FILENO, "libexec.so: [pid: %d] %s; %s%s\n", pid, name_, message, variable);
        }
    }
}
