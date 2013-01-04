// This file is distributed under MIT-LICENSE. See COPYING for details.

#ifndef COMMON_PROTOCOL_H
#define COMMON_PROTOCOL_H

char const *    read_string(int fd);
char const * *  read_string_array(int fd);

void write_string(int fd, char const *);
void write_string_array(int fd, char const * *);

#endif
