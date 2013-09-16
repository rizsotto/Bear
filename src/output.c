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


typedef struct bear_output_stream_t
{
    int fd;
    size_t count;
} bear_output_stream_t;

static void stream_open(bear_output_stream_t *, char const * file);
static void stream_close(bear_output_stream_t *);
static void stream_separator(bear_output_stream_t *);

static char const * get_source_file(bear_output_filter_t const * filter, bear_message_t const * e);


struct bear_output_t
{
    bear_output_stream_t stream;
    bear_output_filter_t const * filter;
};


bear_output_t * bear_open_json_output(char const * file, bear_output_filter_t const * filter)
{
    bear_output_t * handle = malloc(sizeof(bear_output_t));
    if (0 == handle)
    {
        perror("bear: malloc");
        exit(EXIT_FAILURE);
    }

    handle->filter = filter;
    stream_open(& handle->stream, file);

    return handle;
}

void bear_close_json_output(bear_output_t * handle)
{
    stream_close(& handle->stream);

    free((void *)handle);
}

void bear_append_json_output(bear_output_t * handle, bear_message_t const * e)
{
    bear_output_stream_t * const stream = & handle->stream;

    char const * const cmd = bear_strings_fold(bear_json_escape_strings(e->cmd), ' ');
    if (handle->filter)
    {
        char const * const src = get_source_file(handle->filter, e);
        if (src)
        {
            stream_separator(stream);

            dprintf(stream->fd,
                    "{\n"
                    "  \"directory\": \"%s\",\n"
                    "  \"command\": \"%s\",\n"
                    "  \"file\": \"%s\"\n"
                    "}\n",
                    e->cwd, cmd, src);
        }
        free((void *)src);
    }
    else
    {
        stream_separator(stream);

        dprintf(stream->fd,
                "{\n"
                "  \"pid\": \"%d\",\n"
                "  \"ppid\": \"%d\",\n"
                "  \"function\": \"%s\",\n"
                "  \"directory\": \"%s\",\n"
                "  \"command\": \"%s\"\n"
                "}\n",
                e->pid, e->ppid, e->fun, e->cwd, cmd);
    }
    free((void *)cmd);
}

static void stream_open(bear_output_stream_t * handle, char const * file)
{
    handle->count = 0;
    handle->fd = open(file, O_CREAT | O_TRUNC | O_WRONLY, S_IRUSR | S_IWUSR);
    if (-1 == handle->fd)
    {
        perror("bear: open");
        exit(EXIT_FAILURE);
    }

    dprintf(handle->fd, "[\n");
}

static void stream_close(bear_output_stream_t * handle)
{
    dprintf(handle->fd, "]\n");

    if (-1 == close(handle->fd))
    {
        perror("bear: close");
        exit(EXIT_FAILURE);
    }
}

static void stream_separator(bear_output_stream_t * handle)
{
    if (handle->count++)
    {
        dprintf(handle->fd, ",\n");
    }
}


static int is_known_compiler(char const * cmd, char const ** compilers);
static int is_source_file(char const * const arg, char const ** extensions);
static int is_dependency_generation_flag(char const * const arg);

static char const * fix_path(char const * file, char const * cwd);


static char const * get_source_file(bear_output_filter_t const * filter, bear_message_t const * e)
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
