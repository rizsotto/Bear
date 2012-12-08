// This file is distributed under MIT-LICENSE. See COPYING for details.

#ifndef BIN_WRITE_H
#define BIN_WRITE_H

int  cdb_open(char const * file);
void cdb_close(int fd);

struct CDBEntry;

struct CDBEntry * cdb_new();
void cdb_delete(struct CDBEntry * e);

void cdb_read(int fd, struct CDBEntry * e);
void cdb_write(int fd, struct CDBEntry const * e, int debug);

#endif
