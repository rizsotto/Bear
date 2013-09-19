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

#include "filter.h"
#include "stringarray.h"

#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <sys/types.h>
#include <regex.h>


typedef struct regex_list_t
{
    size_t    length;
    regex_t * regexs;
} regex_list_t;

static void compile(char const ** regex, regex_list_t * prepared);
static int  match(regex_list_t const * prepared, char const * input);
static void release(regex_list_t * prepared);


struct bear_output_filter_t
{
    regex_list_t compilers;
    regex_list_t source_files;
    regex_list_t cancel_parameters;
};

static char const * compilers[] =
{
    "^([^/]*/)*c(c|\\+\\+)$",
    "^([^/]*/)*g(cc|\\+\\+)(-4.[012345678]|)$",
    "^([^/]*/)*clang(\\+\\+|)(-3.[01234]|)$",
    "^([^/]*/)*llvm-g(cc|\\+\\+)$",
    0
};

static char const * source_files[] =
{
    ".*\\.[cC]([cC]|\\+\\+|xx|pp|p|)$",
    0
};

static char const * cancel_parameters[] =
{
    "^-M",
    0
};


static char const * fix_path(char const * file, char const * cwd);

bear_output_filter_t * bear_filter_create(char const * file)
{
    bear_output_filter_t * filter = malloc(sizeof(bear_output_filter_t));
    if (0 == filter)
    {
        perror("bear: malloc");
        exit(EXIT_FAILURE);
    }

    compile(compilers, &filter->compilers);
    compile(source_files, &filter->source_files);
    compile(cancel_parameters, &filter->cancel_parameters);

    return filter;
}

void bear_filter_delete(bear_output_filter_t * filter)
{
    release(&filter->compilers);
    release(&filter->source_files);
    release(&filter->cancel_parameters);

    free((void *)filter);
}

char const * bear_filter_source_file(bear_output_filter_t const * filter, bear_message_t const * e)
{
    char const * result = 0;
    // looking for compiler name
    if ((e->cmd) && (e->cmd[0]) && match(&filter->compilers, e->cmd[0]))
    {
        // looking for source file
        char const * const * it = e->cmd;
        for (; *it; ++it)
        {
            if ((0 == result) && match(&filter->source_files, *it))
            {
                result = fix_path(*it, e->cwd);
            }
            else if (match(&filter->cancel_parameters, *it))
            {
                if (result)
                {
                    free((void *)result);
                    result = 0;
                }
                break;
            }
        }
    }
    return result;
}


static void compile(char const ** regex, regex_list_t * prepared)
{
    prepared->length = bear_strings_length(regex);
    prepared->regexs = malloc(prepared->length * sizeof(regex_t));

    char const ** it = regex;
    regex_t * ot = prepared->regexs;
    for (; (it) && (*it); ++it, ++ot)
    {
        if (0 != regcomp(ot, *it, REG_EXTENDED))
        {
            // TODO: use regerror
            perror("bear: regcomp");
            exit(EXIT_FAILURE);
        }
    }
    ot = 0;
}

static int  match(regex_list_t const * prepared, char const * input)
{
    size_t idx = 0;
    for (; idx < prepared->length; ++idx)
    {
        regex_t * ot = prepared->regexs + idx;
        if (0 == regexec(ot, input, 0, 0, 0))
            return 1;
    }
    return 0;
}

static void release(regex_list_t * prepared)
{
    size_t idx = 0;
    for (; idx < prepared->length; ++idx)
    {
        regex_t * ot = prepared->regexs + idx;
        regfree(ot);
    }
    free((void *)prepared->regexs);
}


static char const * fix_path(char const * file, char const * cwd)
{
    char * result = 0;
    if ('/' == file[0])
    {
        result = strdup(file);
        if (0 == result)
        {
            perror("bear: strdup");
            exit(EXIT_FAILURE);
        }
    }
    else
    {
        if (-1 == asprintf(&result, "%s/%s", cwd, file))
        {
            perror("bear: asprintf");
            exit(EXIT_FAILURE);
        }
    }
    return result;
}
