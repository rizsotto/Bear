// This file is distributed under MIT-LICENSE. See COPYING for details.

#ifndef COMMON_PROTOCOL_H
#define COMMON_PROTOCOL_H

#ifdef SERVER
char const *    read_string(int fd);
char const * *  read_string_array(int fd);
#endif

#ifdef CLIENT
void write_string(int fd, char const *);
void write_string_array(int fd, char const * *);
#endif

#endif
