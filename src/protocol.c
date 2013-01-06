// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "protocol.h"
#include "stringarray.h"

#include <string.h>
#include <malloc.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>


#ifdef SERVER
static pid_t read_pid(int fd) {
    pid_t result= 0;
    if (-1 == read(fd, (void *)&result, sizeof(pid_t))) {
        perror("read: pid");
        exit(EXIT_FAILURE);
    }
    return result;
}

static char const * read_string(int fd) {
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

static char const * * read_string_array(int fd) {
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

void bear_read_message(int fd, struct bear_message * e) {
    e->pid = read_pid(fd);
    e->fun = read_string(fd);
    e->cwd = read_string(fd);
    e->cmd = read_string_array(fd);
}
#endif

#ifdef CLIENT
static void write_pid(int fd, pid_t pid) {
    write(fd, (void const *)&pid, sizeof(pid_t));
}

static void write_string(int fd, char const * message) {
    size_t const length = (message) ? strlen(message) : 0;
    write(fd, (void const *)&length, sizeof(size_t));
    if (length > 0) {
        write(fd, (void const *)message, length);
    }
}

static void write_string_array(int fd, char const * * message) {
    size_t const length = sa_length(message);
    write(fd, (void const *)&length, sizeof(size_t));
    size_t it = 0;
    for (; it < length; ++it) {
        write_string(fd, message[it]);
    }
}

void bear_write_message(int fd, struct bear_message const * e) {
    write_pid(fd, e->pid);
    write_string(fd, e->fun);
    write_string(fd, e->cwd);
    write_string_array(fd, e->cmd);
}
#endif
