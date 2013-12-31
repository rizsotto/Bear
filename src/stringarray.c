/*  Copyright (C) 2012, 2013 by László Nagy
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

#include "stringarray.h"

#include <ctype.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

#ifdef CLIENT
char const ** bear_strings_build(char const * const arg, va_list *args)
{
    char const ** result = 0;
    size_t size = 0;
    for (char const * it = arg; it; it = va_arg(*args, char const *))
    {
        result = realloc(result, (size + 1) * sizeof(char const *));
        if (0 == result)
        {
            perror("bear: realloc");
            exit(EXIT_FAILURE);
        }
        char const * copy = strdup(it);
        if (0 == copy)
        {
            perror("bear: strdup");
            exit(EXIT_FAILURE);
        }
        result[size++] = copy;
    }
    result = realloc(result, (size + 1) * sizeof(char const *));
    if (0 == result)
    {
        perror("bear: realloc");
        exit(EXIT_FAILURE);
    }
    result[size++] = 0;

    return result;
}

char const ** bear_strings_copy(char const ** const in)
{
    size_t const size = bear_strings_length(in);

    char const ** const result = malloc((size + 1) * sizeof(char const *));
    if (0 == result)
    {
        perror("bear: malloc");
        exit(EXIT_FAILURE);
    }

    char const ** out_it = result;
    for (char const * const * in_it = in; (in_it) && (*in_it); ++in_it, ++out_it)
    {
        *out_it = strdup(*in_it);
        if (0 == *out_it)
        {
            perror("bear: strdup");
            exit(EXIT_FAILURE);
        }
    }
    *out_it = 0;
    return result;
}

char const ** bear_strings_append(char const ** const in, char const * const e)
{
    if (0 == e)
        return in;

    size_t size = bear_strings_length(in);
    char const ** result = realloc(in, (size + 2) * sizeof(char const *));
    if (0 == result)
    {
        perror("bear: realloc");
        exit(EXIT_FAILURE);
    }
    result[size++] = e;
    result[size++] = 0;
    return result;
}

size_t bear_strings_length(char const * const * const in)
{
    size_t result = 0;
    for (char const * const * it = in; (it) && (*it); ++it)
        ++result;
    return result;
}
#endif

void bear_strings_release(char const ** in)
{
    for (char const * const * it = in; (it) && (*it); ++it)
    {
        free((void *)*it);
    }
    free((void *)in);
}

#ifdef SERVER
char const * bear_strings_find(char const * const * in, char const * const e)
{
    if (0 == e)
        return 0;

    for (char const * const * it = in; (it) && (*it); ++it)
    {
        if (0 == strcmp(e, *it))
            return *it;
    }
    return 0;
}

char const * bear_strings_fold(char const * const * in, char const separator)
{
    // calculate the needed size
    size_t size = 0;
    for (char const * const * it = in; (it) && (*it); ++it)
        size += strlen(*it) + 1;
    // allocate memory once
    char * result = (0 != size) ? malloc(size * sizeof(char)) : 0;
    if (0 == result)
    {
        perror("bear: malloc");
        exit(EXIT_FAILURE);
    }
    // copy each row to the result
    char * result_ptr = result;
    for (char const * const * it = in; (it) && (*it); ++it)
    {
        size_t const it_size = strlen(*it);

        strncpy(result_ptr, *it, it_size);
        result_ptr += it_size;

        *result_ptr++ = separator;
    }
    // replace the last separator to terminating zero
    *--result_ptr = '\0';

    return result;
}
#endif
