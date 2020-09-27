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

#pragma once

#include "Resolver.h"

#include <climits>
#include <string_view>

namespace el {

    /**
     * This class implements the logic how the program execution resolves the
     * executable path from the system environment.
     *
     * The resolution logic implemented as a class to be able to unit test
     * the code and avoid memory allocation.
     */
    class PathResolver {
    public:
        /**
         * Represents the resolution result. The result can be accessed only
         * when the PathResolver is still "alive" on the stack. When the value
         * is NULL, then the error code shall be non zero.
         */
        struct Result {
            const char *return_value;
            const int error_code;

            constexpr explicit operator bool() const noexcept {
                return (return_value != nullptr) && (error_code == 0);
            }
        };

    public:
        explicit PathResolver(el::Resolver const &resolver);

        /**
         * Resolve the executable from system environments.
         *
         * @return resolved executable path as absolute path.
         */
        Result from_current_directory(std::string_view const &file);
        Result from_path(std::string_view const &file, char *const *envp);
        Result from_search_path(std::string_view const &file, const char *search_path);

        PathResolver(PathResolver const &) = delete;
        PathResolver(PathResolver &&) noexcept = delete;

        PathResolver &operator=(PathResolver const &) = delete;
        PathResolver &&operator=(PathResolver &&) noexcept = delete;

    private:
        el::Resolver const &resolver_;
        char result_[PATH_MAX];
    };
}
