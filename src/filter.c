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

#include <libgen.h> // must be before string.h so we get POSIX basename
#include <string.h>
#include <stdlib.h>
#include <stdio.h>


static int is_known_compiler(char const * cmd, char const ** compilers);
static int is_source_file(char const * const arg, char const ** extensions);
static int is_dependency_generation_flag(char const * const arg);

static char const * fix_path(char const * file, char const * cwd);


char const * bear_filter_source_file(bear_output_filter_t const * filter, bear_message_t const * e)
{
    char const * result = 0;
    // looking for compiler name
    if ((e->cmd) && (e->cmd[0]) && is_known_compiler(e->cmd[0], filter->compilers))
    {
        // looking for source file
        char const * const * it = e->cmd;
        for (; *it; ++it)
        {
            if ((0 == result) && (is_source_file(*it, filter->extensions)))
            {
                result = fix_path(*it, e->cwd);
            }
            else if (is_dependency_generation_flag(*it))
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

static int is_known_compiler(char const * cmd, char const ** compilers)
{
    // looking for compiler name
    // have to copy cmd since POSIX basename modifies input
    char * local_cmd = strdup(cmd);
    char * file = basename(local_cmd);
    int result = (bear_strings_find(compilers, file)) ? 1 : 0;
    free(local_cmd);
    return result;
}

static int is_source_file_extension(char const * arg, char const ** extensions);

static int is_source_file(char const * const arg, char const ** extensions)
{
    char const * file_name = strrchr(arg, '/');
    file_name = (file_name) ? file_name : arg;
    char const * extension = strrchr(file_name, '.');
    extension = (extension) ? extension : file_name;

    return is_source_file_extension(extension, extensions);
}

static int is_source_file_extension(char const * arg, char const ** extensions)
{
    return (bear_strings_find(extensions, arg)) ? 1 : 0;
}

static int is_dependency_generation_flag(char const * const arg)
{
    return (2 <= strlen(arg)) && ('-' == arg[0]) && ('M' == arg[1]);
}
