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

#include "protocol.h"
#include "stringarray.h"

#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <sys/un.h>


static int init_socket(char const * file, struct sockaddr_un * addr);

#ifdef SERVER
static ssize_t socket_read(int fd, void * buf, size_t nbyte)
{
    ssize_t sum = 0;
    while (sum != nbyte)
    {
        ssize_t const cur = read(fd, buf + sum, nbyte - sum);
        if (-1 == cur)
        {
            return cur;
        }
        sum += cur;
    }
    return sum;
}

static pid_t read_pid(int fd)
{
    pid_t result = 0;
    if (-1 == socket_read(fd, (void *)&result, sizeof(pid_t)))
    {
        perror("bear: read pid");
        exit(EXIT_FAILURE);
    }
    return result;
}

static char const * read_string(int fd)
{
    size_t length = 0;
    if (-1 == socket_read(fd, (void *)&length, sizeof(size_t)))
    {
        perror("bear: read string length");
        exit(EXIT_FAILURE);
    }
    char * result = malloc((length + 1) * sizeof(char));
    if (0 == result)
    {
        perror("bear: malloc");
        exit(EXIT_FAILURE);
    }
    if (length > 0)
    {
        if (-1 == socket_read(fd, (void *)result, length))
        {
            perror("bear: read string value");
            exit(EXIT_FAILURE);
        }
    }
    result[length] = '\0';
    return result;
}

static char const * * read_string_array(int fd)
{
    size_t length = 0;
    if (-1 == socket_read(fd, (void *)&length, sizeof(size_t)))
    {
        perror("bear: read string array length");
        exit(EXIT_FAILURE);
    }
    char const * * result = malloc((length + 1) * sizeof(char const *));
    if (0 == result)
    {
        perror("bear: malloc");
        exit(EXIT_FAILURE);
    }
    for (size_t it = 0; it < length; ++it)
    {
        result[it] = read_string(fd);
    }
    result[length] = 0;
    return result;
}

void bear_read_message(int fd, bear_message_t * e)
{
    e->pid = read_pid(fd);
    e->ppid = read_pid(fd);
    e->fun = read_string(fd);
    e->cwd = read_string(fd);
    e->cmd = read_string_array(fd);
}

void bear_free_message(bear_message_t * e)
{
    e->pid = 0;
    e->ppid = 0;
    free((void *)e->fun);
    e->fun = 0;
    free((void *)e->cwd);
    e->cwd = 0;
    bear_strings_release(e->cmd);
    e->cmd = 0;
}

int bear_create_unix_socket(char const * file)
{
    struct sockaddr_un addr;
    int s = init_socket(file, &addr);
    if (-1 == bind(s, (struct sockaddr *)&addr, sizeof(struct sockaddr_un)))
    {
        perror("bear: bind");
        exit(EXIT_FAILURE);
    }
    if (-1 == listen(s, 0))
    {
        perror("bear: listen");
        exit(EXIT_FAILURE);
    }
    return s;
}

int bear_accept_message(int s, bear_message_t * msg)
{
    int connection = accept(s, 0, 0);
    if (-1 != connection)
    {
        bear_read_message(connection, msg);
        close(connection);
        return 1;
    }
    return 0;
}
#endif

#ifdef CLIENT
static ssize_t socket_write(int fd, const void *buf, size_t nbyte)
{
    ssize_t sum = 0;
    while (sum != nbyte)
    {
        ssize_t const cur = write(fd, buf + sum, nbyte - sum);
        if (-1 == cur)
        {
            return cur;
        }
        sum += cur;
    }
    return sum;
}

static void write_pid(int fd, pid_t pid)
{
    socket_write(fd, (void const *)&pid, sizeof(pid_t));
}

static void write_string(int fd, char const * message)
{
    size_t const length = (message) ? strlen(message) : 0;
    socket_write(fd, (void const *)&length, sizeof(size_t));
    if (length > 0)
    {
        socket_write(fd, (void const *)message, length);
    }
}

static void write_string_array(int fd, char const * const * message)
{
    size_t const length = bear_strings_length(message);
    socket_write(fd, (void const *)&length, sizeof(size_t));
    for (size_t it = 0; it < length; ++it)
    {
        write_string(fd, message[it]);
    }
}

void bear_write_message(int fd, bear_message_t const * e)
{
    write_pid(fd, e->pid);
    write_pid(fd, e->ppid);
    write_string(fd, e->fun);
    write_string(fd, e->cwd);
    write_string_array(fd, e->cmd);
}

void bear_send_message(char const * file, bear_message_t const * msg)
{
    struct sockaddr_un addr;
    int s = init_socket(file, &addr);
    if (-1 == connect(s, (struct sockaddr *)&addr, sizeof(struct sockaddr_un)))
    {
        perror("bear: connect");
        exit(EXIT_FAILURE);
    }
    bear_write_message(s, msg);
    close(s);
}
#endif

static int init_socket(char const * file, struct sockaddr_un * addr)
{
    int const s = socket(AF_UNIX, SOCK_STREAM, 0);
    if (-1 == s)
    {
        perror("bear: socket");
        exit(EXIT_FAILURE);
    }
    memset((void *)addr, 0, sizeof(struct sockaddr_un));
    addr->sun_family = AF_UNIX;
    strncpy(addr->sun_path, file, sizeof(addr->sun_path) - 1);
    return s;
}
