/*  Copyright (C) 2012-2022 by László Nagy
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

#include "report/libexec/Logger.h"

#include <ctime>
#include <cstdio>
#include <unistd.h>

namespace {

    el::log::Level LEVEL = el::log::SILENT;

    void verbose_message(char const* name, char const* message, char const* variable)
    {
        struct timespec ts { 0, 0 };
        if (::clock_gettime(CLOCK_REALTIME, &ts) == -1) {
            // ignore failure, default values will be good
        }
        struct tm local_time {};
        ::localtime_r(&ts.tv_sec, &local_time);
        const unsigned long micros = ts.tv_nsec / 1000;
        const pid_t pid = ::getpid();
        ::dprintf(STDERR_FILENO, "[%02d:%02d:%02d.%06ld, el, %d] %s; %s%s\n",
            local_time.tm_hour, local_time.tm_min, local_time.tm_sec, micros, pid, name, message, variable);
    }
}

namespace el::log {

    void set(Level level)
    {
        LEVEL = level;
        fsync(STDERR_FILENO);
    }

    void Logger::debug(char const* message) const noexcept
    {
        this->debug(message, "");
    }

    void Logger::debug(char const* message, char const* variable) const noexcept
    {
        if (el::log::VERBOSE == LEVEL) {
            verbose_message(name_, message, variable);
        }
    }

    void Logger::warning(char const* message) const noexcept
    {
        if (el::log::VERBOSE == LEVEL) {
            verbose_message(name_, message, "");
        } else {
            dprintf(STDERR_FILENO, "libexec.so: %s; %s\n", name_, message);
        }
    }
}
