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
#endif

#ifdef CLIENT
void bear_write_message(int fd, struct bear_message const * e);
#endif

#endif
