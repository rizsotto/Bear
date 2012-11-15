// This file is distributed under MIT-LICENSE. See COPYING for details.

#ifndef BIN_WRITE_H
#define BIN_WRITE_H

#include <stddef.h>

int  cdb_open(char const * file);
void cdb_close(int handle);

struct CDBEntry {
    char const * cwd;
    char const * cmd;
    char const * src;
};

void cdb_read(int fd, struct CDBEntry * e);
int  cdb_filter(struct CDBEntry * e);
void cdb_write(int handle, struct CDBEntry const * e, size_t count);
void cdb_finish(struct CDBEntry * e);

#endif
