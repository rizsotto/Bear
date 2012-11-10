// This file is distributed under MIT-LICENSE. See COPYING for details.

#include <unistd.h>
#include <string.h>
#include <malloc.h>
#include <stdlib.h>
#include <stdio.h>


static char const * read_string(int in);

void copy(int in, int out) {
    char const * const cwd = read_string(in);
    char const * const cmd = read_string(in);
    // FIXME: do filtering and formating
    write(out, cwd, strlen(cwd));
    write(out, "\n", 1);
    write(out, cmd, strlen(cmd));
    write(out, "\n", 1);
    free((void *)cwd);
    free((void *)cmd);
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

