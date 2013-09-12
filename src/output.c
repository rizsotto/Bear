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


struct bear_output
{
    int fd;
    size_t count;
    struct bear_configuration const * config;
};


struct bear_output * bear_open_json_output(char const * file, struct bear_configuration const * config)
{
    struct bear_output * handle = malloc(sizeof(struct bear_output));
    if (0 == handle)
    {
        perror("bear: malloc");
        exit(EXIT_FAILURE);
    }

    handle->count = 0;
    handle->config = config;
    handle->fd = open(file, O_CREAT | O_TRUNC | O_WRONLY, S_IRUSR | S_IWUSR);
    if (-1 == handle->fd)
    {
        perror("bear: open");
        exit(EXIT_FAILURE);
    }

    dprintf(handle->fd, "[\n");

    return handle;
}

void bear_close_json_output(struct bear_output * handle)
{
    dprintf(handle->fd, "]\n");
    close(handle->fd);
    free((void *)handle);
}

static char const * get_source_file(char const * * cmd, char const * cwd, struct bear_configuration const * config);

void bear_append_json_output(struct bear_output * handle, struct bear_message const * e)
{
    char const * const src = get_source_file(e->cmd, e->cwd, handle->config);
    char const * const cmd = bear_strings_fold(bear_json_escape_strings(e->cmd), ' ');
    if (handle->config->debug)
    {
        if (handle->count++)
        {
            dprintf(handle->fd, ",\n");
        }
        dprintf(handle->fd,
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
        if (handle->count++)
        {
            dprintf(handle->fd, ",\n");
        }
        dprintf(handle->fd,
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


static int is_known_compiler(char const * cmd, char const ** compilers);
static int is_source_file(char const * const arg, char const ** extensions);
static int is_dependency_generation_flag(char const * const arg);

static char const * fix_path(char const * file, char const * cwd);


static char const * get_source_file(char const * * args, char const * cwd, struct bear_configuration const * config)
{
    char const * result = 0;
    // looking for compiler name
    if ((args) && (args[0]) && is_known_compiler(args[0], config->compilers))
    {
        // looking for source file
        char const * const * it = args;
        for (; *it; ++it)
        {
            if ((0 == result) && (is_source_file(*it, config->extensions)))
            {
                result = fix_path(*it, cwd);
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
