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

static int symbolic_escape(int);
static int needs_numeric_escape(int);

char const * bear_json_escape_string(char const * raw)
{
    size_t const length = (raw) ? strlen(raw) : 0;
    size_t const symbolic = count(raw, raw + length, symbolic_escape);
    size_t const numeric = count(raw, raw + length, needs_numeric_escape);

    if ((0 == symbolic) && (0 == numeric))
    {
        return 0;
    }

    char * const result = malloc(length + symbolic + numeric * 5 + 1);
    if (0 == result)
    {
        perror("bear: malloc");
        exit(EXIT_FAILURE);
    }
    char * it = result;
    for (; (raw) && (*raw); ++raw)
    {
        if (needs_numeric_escape(*raw))
        {
            sprintf(it, "\\u%04x", *raw);
            it += 6;
        }
        else
        {
            char escape = symbolic_escape(*raw);
            if (escape)
            {
                *it++ = '\\';
                *it++ = escape;
            }
            else
            {
                *it++ = *raw;
            }
        }
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

static int symbolic_escape(int c)
{
    switch (c)
    {
    case '\\': return '\\';
    case '\"': return '\"';
    case '\b': return 'b';
    case '\f': return 'f';
    case '\n': return 'n';
    case '\r': return 'r';
    case '\t': return 't';
    }
    return 0;
}

static int needs_numeric_escape(int c)
{
    return c > 0 && c < ' ' && !symbolic_escape(c);
}
