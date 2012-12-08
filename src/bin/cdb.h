// This file is distributed under MIT-LICENSE. See COPYING for details.

#ifndef BIN_WRITE_H
#define BIN_WRITE_H

#include <stddef.h>

int  cdb_open(char const * file);
void cdb_close(int handle);

struct CDBEntry;

struct CDBEntry * cdb_new();
void cdb_delete(struct CDBEntry * e);

void cdb_read(int fd, struct CDBEntry * e);
int  cdb_filter(struct CDBEntry * e);
void cdb_write(int handle, struct CDBEntry const * e, size_t count);

#endif
