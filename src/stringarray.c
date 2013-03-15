// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "stringarray.h"

#include <ctype.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

#ifdef CLIENT
char const ** bear_strings_copy(char const ** const in)
{
    size_t const size = bear_strings_length(in);

    char const ** result =
        (char const **)malloc((size + 1) * sizeof(char const *));
    if (0 == result)
    {
        perror("malloc");
        exit(EXIT_FAILURE);
    }
    result[size] = 0;

    char const * const * in_it = in;
    char const * * out_it = result;
    for (; *in_it; ++in_it, ++out_it)
    {
        *out_it = strdup(*in_it);
        if (0 == *out_it)
        {
            perror("strdup");
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
    for (; it; it = va_arg(args, char const *))
    {
        result = (char const **)realloc(result, (size + 1) * sizeof(char const *));
        char const * copy = strdup(it);
        if (0 == copy)
        {
            perror("strdup");
            exit(EXIT_FAILURE);
        }
        result[size++] = copy;
    }
    result = (char const **)realloc(result, (size + 1) * sizeof(char const *));
    result[size++] = 0;

    return result;
}
#endif

void bear_strings_release(char const ** in)
{
    char const * const * it = in;
    for (; (in) && (*it); ++it)
    {
        free((void *)*it);
    }
    free((void *)in);
    in = 0;
}

char const ** bear_strings_append(char const ** const in, char const * const e)
{
    if (0 == e)
    {
        return in;
    }
    size_t size = bear_strings_length(in);
    char const ** result = (char const **)realloc(in, (size + 2) * sizeof(char const *));
    if (0 == result)
    {
        perror("realloc");
        exit(EXIT_FAILURE);
    }
    result[size++] = e;
    result[size++] = 0;
    return result;
}

char const ** bear_strings_remove(char const ** const in, char const * const e)
{
    if (0 == e)
    {
        return in;
    }
    char const * * it = in;
    int action = 0;
    for (; (in) && (*it); ++it)
    {
        if ((*it) == e)
        {
            ++action;
        }
        if (action)
        {
            char const * * next = it + 1;
            *it = *next;
        }
    }
    // now resize the array
    size_t size = bear_strings_length(in);
    char const ** result = (char const **)realloc(in, (size + 1) * sizeof(char const *));
    if (0 == result)
    {
        perror("realloc");
        exit(EXIT_FAILURE);
    }
    return result;
}

size_t bear_strings_length(char const * const * const in)
{
    size_t result = 0;
    char const * const * it = in;
    for (; (in) && (*it); ++it, ++result)
        ;
    return result;
}

int bear_strings_find(char const * const * in, char const * const e)
{
    if (0 == e)
    {
        return 0;
    }

    char const * const * it = in;
    for (; (in) && (*it); ++it)
    {
        if (0 == strcmp(e, *it))
        {
            return 1;
        }
    }
    return 0;
}

#ifdef SERVER
char const * bear_strings_fold(char const * const * in, char const separator)
{
    char * acc = 0;
    size_t acc_size = 0;

    char const * const * it = in;
    for (; *it; ++it)
    {
        size_t const sep = (in == it) ? 0 : 1;
        size_t const it_size = strlen(*it);
        acc = (char *)realloc(acc, acc_size + sep + it_size);
        if (0 == acc)
        {
            perror("realloc");
            exit(EXIT_FAILURE);
        }
        if (sep)
        {
            acc[acc_size++] = separator;
        }
        strncpy((acc + acc_size), *it, it_size);
        acc_size += it_size;
    }
    acc = (char *)realloc(acc, acc_size + 1);
    if (0 == acc)
    {
        perror("realloc");
        exit(EXIT_FAILURE);
    }
    acc[acc_size++] = '\0';
    return acc;
}
#endif
