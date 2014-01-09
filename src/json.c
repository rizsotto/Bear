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

#include "json.h"
#include "stringarray.h"

#include <string.h>
#include <ctype.h>
#include <stdlib.h>
#include <stdio.h>


char const * * bear_json_escape_strings(char const * * raw)
{
    for (char const * * it = raw; (raw) && (*it); ++it)
    {
        char const * const new = bear_json_escape_string(*it);
        if (new)
        {
            char const * const tmp = *it;
            *it = new;
            free((void *)tmp);
        }
    }
    return raw;
}

static size_t count(char const * const begin,
                    char const * const end,
                    int(*fp)(int));

static int needs_escape(int);

char const * bear_json_escape_string(char const * raw)
{
    size_t const length = (raw) ? strlen(raw) : 0;
    size_t const spaces = count(raw, raw + length, isspace);
    size_t const json = count(raw, raw + length, needs_escape);

    if ((0 == spaces) && (0 == json))
    {
        return 0;
    }

    char * const result = malloc(length + ((0 != spaces) * 4) + json + 1);
    if (0 == result)
    {
        perror("bear: malloc");
        exit(EXIT_FAILURE);
    }
    char * it = result;
    if (spaces)
    {
        *it++ = '\\';
        *it++ = '\"';
    }
    for (; (raw) && (*raw); ++raw)
    {
        if (needs_escape(*raw))
        {
            *it++ = '\\';
        }
        *it++ = isspace(*raw) ? ' ' : *raw;
    }
    if (spaces)
    {
        *it++ = '\\';
        *it++ = '\"';
    }
    *it = '\0';
    return result;
}

static size_t count(char const * const begin,
                    char const * const end,
                    int (*fp)(int))
{
    size_t result = 0;
    for (char const * it = begin; it != end; ++it)
    {
        if (fp(*it))
            ++result;
    }
    return result;
}

static int needs_escape(int c)
{
    switch (c)
    {
    case '\\':
    case '\"':
        return 1;
    }
    return 0;
}
