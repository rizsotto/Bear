// This file is distributed under MIT-LICENSE. See COPYING for details.

#ifndef BEAR_PROTOCOL_H
#define BEAR_PROTOCOL_H

#include <unistd.h>

struct bear_message {
    pid_t pid;
    char const * fun;
    char const * cwd;
    char const * * cmd;
};

#ifdef SERVER
void bear_read_message(int fd, struct bear_message * e);
void bear_free_message(struct bear_message * e);

int bear_create_unix_socket(char const * socket);
int bear_accept_message(int fd, struct bear_message * e);
#endif

#ifdef CLIENT
void bear_write_message(int fd, struct bear_message const * e);

void bear_send_message(char const * socket, struct bear_message const * e);
#endif

#endif
