/*  Copyright (C) 2012-2014 by László Nagy
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

#include <stddef.h>

#ifdef CLIENT
#include <stdarg.h>
char const ** bear_strings_build(char const * arg, va_list *ap);

char const ** bear_strings_copy(char const ** const in);
char const ** bear_strings_append(char const ** in, char const * e);

size_t        bear_strings_length(char const * const * in);
#endif

void          bear_strings_release(char const **);

#ifdef SERVER
char const *  bear_strings_find(char const * const * in, char const * e);
char const *  bear_strings_fold(char const * const * in, char const sep);
char const * * bear_strings_transform(char const * * in,
                                      char const * (* transform_one)(char const *));
#endif
