// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "cdb.h"

#include <unistd.h>
#include <string.h>
#include <malloc.h>
#include <stdlib.h>
#include <stdio.h>
#include <sys/stat.h>
#include <fcntl.h>


static char const * read_string(int in);


int cdb_open(char const * file) {
    int ofd = open(file, O_CREAT|O_RDWR, S_IRUSR|S_IWUSR);
    if (-1 == ofd) {
        perror("open");
        exit(EXIT_FAILURE);
    }
    return ofd;
}

void cdb_copy(int ofd, int ifd) {
    char const * const cwd = read_string(ifd);
    char const * const cmd = read_string(ifd);
    // FIXME: do filterifdg and formatifdg
    write(ofd, cwd, strlen(cwd));
    write(ofd, "\n", 1);
    write(ofd, cmd, strlen(cmd));
    write(ofd, "\n", 1);
    free((void *)cwd);
    free((void *)cmd);
}

void cdb_close(int ofd) {
    close(ofd);
}

static char const * read_string(int in) {
    size_t length = 0;
    if (-1 == read(in, (void *)&length, sizeof(size_t))) {
        perror("read: header");
        exit(EXIT_FAILURE);
    }
    if (length > 0) {
        char * result = malloc(length + 1);
        if (-1 == read(in, (void *)result, length)) {
            free(result);
            perror("read: message");
            exit(EXIT_FAILURE);
        }
        result[length] = '\0';
        return result;
    }
    return "";
}

