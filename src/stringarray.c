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
char const ** bear_strings_copy(char const ** const in)
{
    size_t const size = bear_strings_length(in);

    char const ** result = malloc((size + 1) * sizeof(char const *));
    if (0 == result)
    {
        perror("bear: malloc");
        exit(EXIT_FAILURE);
    }
    result[size] = 0;

    char const * const * in_it = in;
    char const * * out_it = result;
    for (; (in_it) && (*in_it); ++in_it, ++out_it)
    {
        *out_it = strdup(*in_it);
        if (0 == *out_it)
        {
            perror("bear: strdup");
            exit(EXIT_FAILURE);
        }
    }
    return result;
}

char const ** bear_strings_build(char const * const arg, va_list args)
{
    char const ** result = 0;
    char const * it = arg;
    size_t size = 0;
    size_t i = 0;
    for (; it; it = va_arg(args, char const *))
    {
        char const** tmp = realloc(result, (size + 1) * sizeof(char const *));
        if (0 == tmp)
        {
            for (i = 0; i < size; i++) {
              free((char**) result[i]);
            }
            free(result);
            perror("bear: realloc");
            exit(EXIT_FAILURE);
        }
        result = tmp;
        char const * copy = strdup(it);
        if (0 == copy)
        {
            for (i = 0; i < size; i++) {
              free((char**) result[i]);
            }
            free(result);
            perror("bear: strdup");
            exit(EXIT_FAILURE);
        }
        result[size++] = copy;
    }
    char const** tmp = realloc(result, (size + 1) * sizeof(char const *));
    if (0 == tmp)
    {
        for (i = 0; i < size; i++) {
          free((char**) result[i]);
        }
        free(result);
        perror("bear: realloc");
        exit(EXIT_FAILURE);
    }
    result = tmp;
    result[size++] = 0;

    return result;
}
#endif

void bear_strings_release(char const ** in)
{
    char const * const * it = in;
    for (; (it) && (*it); ++it)
    {
        free((void *)*it);
    }
    free((void *)in);
    *in = 0;
}

char const ** bear_strings_append(char const ** const in, char const * const e)
{
    if (0 == e)
        return in;

    size_t size = bear_strings_length(in);
    char const ** result = realloc(in, (size + 2) * sizeof(char const *));
    if (0 == result)
    {
        free(in);
        perror("bear: realloc");
        exit(EXIT_FAILURE);
    }
    result[size++] = e;
    result[size++] = 0;
    return result;
}

char const ** bear_strings_remove(char const ** const in, char const * const e)
{
    if (0 == e)
        return in;

    int action = 0;
    char const * * it = in;
    for (; (it) && (*it); ++it)
    {
        if ((*it) == e)
            ++action;

        if (action)
            *it = *(it + action);
    }
    // resize the array when needed
    if (action)
    {
        size_t size = bear_strings_length(in);
        char const ** result = realloc(in, (size + 1) * sizeof(char const *));
        if (0 == result)
        {
            free(in);
            perror("bear: realloc");
            exit(EXIT_FAILURE);
        }
        return result;
    }
    return in;
}

size_t bear_strings_length(char const * const * const in)
{
    size_t result = 0;
    char const * const * it = in;
    for (; (it) && (*it); ++it, ++result)
        ;
    return result;
}

int bear_strings_find(char const * const * in, char const * const e)
{
    if (0 == e)
        return 0;

    char const * const * it = in;
    for (; (it) && (*it); ++it)
    {
        if (0 == strcmp(e, *it))
            return 1;
    }
    return 0;
}

#ifdef SERVER
char const * bear_strings_fold(char const * const * in, char const separator)
{
    char * acc = 0;
    size_t acc_size = 0;

    char const * const * it = in;
    for (; (it) && (*it); ++it)
    {
        size_t const sep = (in == it) ? 0 : 1;
        size_t const it_size = strlen(*it);
        char* tmp = realloc(acc, acc_size + sep + it_size);
        if (0 == tmp)
        {
            free(acc);
            perror("bear: realloc");
            exit(EXIT_FAILURE);
        }
        acc = tmp;
        if (sep)
        {
            acc[acc_size++] = separator;
        }
        strncpy((acc + acc_size), *it, it_size);
        acc_size += it_size;
    }
    char* tmp = realloc(acc, acc_size + 1);
    if (0 == tmp)
    {
        free(acc);
        perror("bear: realloc");
        exit(EXIT_FAILURE);
    }
    acc = tmp;
    acc[acc_size++] = '\0';
    return acc;
}
#endif
