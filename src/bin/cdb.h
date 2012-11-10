// This file is distributed under MIT-LICENSE. See COPYING for details.

#ifndef BIN_WRITE_H
#define BIN_WRITE_H

int  cdb_open(char const * file);
void cdb_copy(int ofd, int ifd);
void cdb_close(int ofd);

#endif
