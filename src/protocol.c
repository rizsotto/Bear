// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "protocol.h"
#include "stringarray.h"

#include <string.h>
#include <malloc.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>


#ifdef SERVER
char const * read_string(int fd) {
    size_t length = 0;
    if (-1 == read(fd, (void *)&length, sizeof(size_t))) {
        perror("read: string length");
        exit(EXIT_FAILURE);
    }
    char * result = malloc((length + 1) * sizeof(char));
    if (0 == result) {
        perror("malloc");
        exit(EXIT_FAILURE);
    }
    if (length > 0) {
        if (-1 == read(fd, (void *)result, length)) {
            perror("read: string value");
            exit(EXIT_FAILURE);
        }
    }
    result[length] = '\0';
    return result;
}

char const * * read_string_array(int fd) {
    size_t length = 0;
    if (-1 == read(fd, (void *)&length, sizeof(size_t))) {
        perror("read: string array length");
        exit(EXIT_FAILURE);
    }
    char const * * result =
        (char const * *)malloc((length + 1) * sizeof(char const *));
    if (0 == result) {
        perror("malloc");
        exit(EXIT_FAILURE);
    }
    size_t it = 0;
    for (; it < length; ++it) {
        result[it] = read_string(fd);
    }
    result[length] = 0;
    return result;
}
#endif

#ifdef CLIENT
void write_string(int fd, char const * message) {
    size_t const length = (message) ? strlen(message) : 0;
    write(fd, (void const *)&length, sizeof(size_t));
    if (length > 0) {
        write(fd, (void const *)message, length);
    }
}

void write_string_array(int fd, char const * * message) {
    size_t const length = sa_length(message);
    write(fd, (void const *)&length, sizeof(size_t));
    size_t it = 0;
    for (; it < length; ++it) {
        write_string(fd, message[it]);
    }
}
#endif
