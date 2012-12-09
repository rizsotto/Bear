// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "protocol.h"

#include <malloc.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

size_t message_length(char const *);

char const * read_string(int fd) {
    size_t length = 0;
    if (-1 == read(fd, (void *)&length, sizeof(size_t))) {
        perror("read: header");
        exit(EXIT_FAILURE);
    }
    char * result = malloc(length + 1);
    if (0 == result) {
        perror("malloc");
        exit(EXIT_FAILURE);
    }
    if (length > 0) {
        if (-1 == read(fd, (void *)result, length)) {
            perror("read: message");
            free(result);
            exit(EXIT_FAILURE);
        }
    }
    result[length] = '\0';
    return result;
}

void write_string(int fd, char const * message) {
    size_t const length = message_length(message);
    write(fd, (void const *)&length, sizeof(size_t));
    if (length > 0) {
        write(fd, (void const *)message, length);
    }
}

size_t message_length(char const * msg) {
    size_t result = 0;
    for (; (msg) && (*msg); ++msg, ++result)
        ;
    return result;
}

