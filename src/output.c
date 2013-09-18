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
#include "json.h"

#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <fcntl.h>
#include <sys/stat.h>


typedef struct bear_output_stream_t
{
    int fd;
    size_t count;
} bear_output_stream_t;

static void stream_open(bear_output_stream_t *, char const * file);
static void stream_close(bear_output_stream_t *);
static void stream_separator(bear_output_stream_t *);


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
        char const * const src = bear_filter_source_file(handle->filter, e);
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

