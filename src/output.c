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

#include "output.h"
#include "stringarray.h"
#include "protocol.h"
#include "json.h"

#include <unistd.h>
#include <libgen.h> // must be before string.h so we get POSIX basename
#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <stddef.h>


static size_t count = 0;

int bear_open_json_output(char const * file)
{
    int fd = open(file, O_CREAT | O_TRUNC | O_WRONLY, S_IRUSR | S_IWUSR);
    if (-1 == fd)
    {
        perror("bear: open");
        exit(EXIT_FAILURE);
    }
    dprintf(fd, "[\n");
    count = 0;
    return fd;
}

void bear_close_json_output(int fd)
{
    dprintf(fd, "]\n");
    close(fd);
}

static char const * get_source_file(char const * * cmd, char const * cwd);

void bear_append_json_output(int fd, struct bear_message const * e, int debug)
{
    char const * src = get_source_file(e->cmd, e->cwd);
    char const * const cmd = bear_strings_fold(bear_json_escape_strings(e->cmd), ' ');
    if (debug)
    {
        if (count++)
        {
            dprintf(fd, ",\n");
        }
        dprintf(fd,
                "{\n"
                "  \"pid\": \"%d\",\n"
                "  \"ppid\": \"%d\",\n"
                "  \"function\": \"%s\",\n"
                "  \"directory\": \"%s\",\n"
                "  \"command\": \"%s\"\n"
                "}\n",
                e->pid, e->ppid, e->fun, e->cwd, cmd);
    }
    else if (src)
    {
        if (count++)
        {
            dprintf(fd, ",\n");
        }
        dprintf(fd,
                "{\n"
                "  \"directory\": \"%s\",\n"
                "  \"command\": \"%s\",\n"
                "  \"file\": \"%s\"\n"
                "}\n",
                e->cwd, cmd, src);
    }
    free((void *)cmd);
    free((void *)src);
}


static int is_known_compiler(char const * cmd);
static int is_source_file(char const * const arg);
static int is_dependency_generation_flag(char const * const arg);

static char const * fix_path(char const * file, char const * cwd);


static char const * get_source_file(char const * * args, char const * cwd)
{
    char const * result = 0;
    // looking for compiler name
    if ((args) && (args[0]) && is_known_compiler(args[0]))
    {
        // looking for source file
        char const * const * it = args;
        for (; *it; ++it)
        {
            if (is_source_file(*it))
            {
                result = fix_path(*it, cwd);
            }
            else if (is_dependency_generation_flag(*it))
            {
                result = 0;
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

static char const * const compilers[] =
{
    "cc",
    "gcc",
    "gcc-4.1",
    "gcc-4.2",
    "gcc-4.3",
    "gcc-4.4",
    "gcc-4.5",
    "gcc-4.6",
    "gcc-4.7",
    "gcc-4.8",
    "llvm-gcc",
    "clang",
    "clang-3.0",
    "clang-3.1",
    "clang-3.2",
    "clang-3.3",
    "clang-3.4",
    "c++",
    "g++",
    "g++-4.1",
    "g++-4.2",
    "g++-4.3",
    "g++-4.4",
    "g++-4.5",
    "g++-4.6",
    "g++-4.7",
    "g++-4.8",
    "llvm-g++",
    "clang++",
    0
};

static int is_known_compiler(char const * cmd)
{
    // looking for compiler name
    // have to copy cmd since POSIX basename modifies input
    char * local_cmd = strdup(cmd);
    char * file = basename(local_cmd);
    int result = bear_strings_find(compilers, file);
    free(local_cmd);
    return result;
}

static int is_source_file_extension(char const * arg);

static int is_source_file(char const * const arg)
{
    char const * file_name = strrchr(arg, '/');
    file_name = (file_name) ? file_name : arg;
    char const * extension = strrchr(file_name, '.');
    extension = (extension) ? extension : file_name;

    return is_source_file_extension(extension);
}

static char const * const extensions[] =
{
    ".c",
    ".C",
    ".cc",
    ".cxx",
    ".c++",
    ".C++",
    ".cpp",
    ".cp",
    ".i",
    ".ii",
    ".m",
    ".S",
    0
};

static int is_source_file_extension(char const * arg)
{
    return bear_strings_find(extensions, arg);
}

static int is_dependency_generation_flag(char const * const arg)
{
    return (2 <= strlen(arg)) && ('-' == arg[0]) && ('M' == arg[1]);
}

static void print_array(char const * const * const in)
{
    char const * const * it = in;
    for (; *it; ++it)
    {
        printf("  %s\n",*it);
    }
}

void bear_print_known_compilers()
{
    print_array(compilers);
}

void bear_print_known_extensions()
{
    print_array(extensions);
}
