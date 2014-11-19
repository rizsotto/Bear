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

#include "stringtransform.h"
#include "stringarray.h"

#include <string.h>
#include <ctype.h>
#include <stdlib.h>
#include <stdio.h>


static size_t count(char const * const begin,
                    char const * const end,
                    int(*fp)(int));

static int json_symbolic_escape(int);
static int needs_json_numeric_escape(int);
static int needs_shell_escape(int c);
static int needs_shell_quote(int c);

char const * bear_string_json_escape(char const * raw)
{
    size_t const length = (raw) ? strlen(raw) : 0;
    size_t const symbolic = count(raw, raw + length, json_symbolic_escape);
    size_t const numeric = count(raw, raw + length, needs_json_numeric_escape);

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
        if (needs_json_numeric_escape(*raw))
        {
            sprintf(it, "\\u%04x", *raw);
            it += 6;
        }
        else
        {
            char escape = json_symbolic_escape(*raw);
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

char const * bear_string_shell_escape(char const * raw)
{
    /* This performs minimal shell escaping and/or quoting, as per the JSON
       Compilation Database Format Specification. Only quotes and backslashes
       are treated as special, as well as blanks and newlines (which would
       delimit arguments if left as-is).

       Quoting is only required for newlines (they can't be escaped) and empty
       arguments, but we also do it for blanks, because that looks better than
       escaping, and because it makes the logic simpler (blanks can't be
       escaped inside quotes). */

    size_t const length = (raw) ? strlen(raw) : 0;
    size_t const escaped = count(raw, raw + length, needs_shell_escape);
    size_t const quoted = count(raw, raw + length, needs_shell_quote);
    int const need_quoting = quoted != 0 || length == 0;

    if (0 == escaped && !need_quoting)
    {
        return 0;
    }

    char * const result = malloc(length + escaped + (need_quoting ? 2 : 0) + 1);
    if (0 == result)
    {
        perror("bear: malloc");
        exit(EXIT_FAILURE);
    }
    char * it = result;

    if (need_quoting) *it++ = '\"';

    for (; (raw) && (*raw); ++raw)
    {
        if (needs_shell_escape(*raw)) *it++ = '\\';
        *it++ = *raw;
    }

    if (need_quoting) *it++ = '\"';

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

static int json_symbolic_escape(int c)
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

static int needs_json_numeric_escape(int c)
{
    return c > 0 && c < ' ' && !json_symbolic_escape(c);
}

static int needs_shell_escape(int c)
{
    switch (c)
    {
    case '\\': return 1;
    case '\"': return 1;
    }
    return 0;
}

static int needs_shell_quote(int c)
{
    return isblank(c) || c == '\n';
}
