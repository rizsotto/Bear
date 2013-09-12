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

#pragma once

#include <unistd.h>

typedef struct bear_message_t
{
    pid_t pid;
    pid_t ppid;
    char const * fun;
    char const * cwd;
    char const * * cmd;
} bear_message_t;

#ifdef SERVER
void bear_read_message(int fd, bear_message_t * e);
void bear_free_message(bear_message_t * e);

int bear_create_unix_socket(char const * socket);
int bear_accept_message(int fd, bear_message_t * e);
#endif

#ifdef CLIENT
void bear_write_message(int fd, bear_message_t const * e);

void bear_send_message(char const * socket, bear_message_t const * e);
#endif
