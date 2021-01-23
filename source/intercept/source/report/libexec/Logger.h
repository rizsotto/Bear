/*  Copyright (C) 2012-2021 by László Nagy
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

namespace el::log {

    enum Level {
        SILENT = 0,
        VERBOSE = 1
    };

    // Not MT safe
    void set(Level);

    class Logger {
    public:
        constexpr explicit Logger(const char *name) noexcept;

        ~Logger() noexcept = default;

        void debug(char const *message) const noexcept;
        void debug(char const *message, char const *variable) const noexcept;

        void warning(char const *message) const noexcept;

    private:
        const char *name_;
    };

    inline constexpr
    Logger::Logger(const char *name) noexcept
            : name_(name)
    { }
}
