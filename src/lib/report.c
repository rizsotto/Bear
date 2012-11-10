// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "report.h"

#include <unistd.h>
#include <malloc.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <sys/un.h>

static void write_cwd(int fd);
static void write_call(int fd, char const * const argv[]);

void report_call(char const *method, char * const argv[]) {
    // get output file name
    char * const out = getenv("BEAR_OUTPUT");
    // connect to server
    if (out) {
        int s = socket(AF_UNIX, SOCK_STREAM, 0);
        if (-1 == s) {
            return;
        }
        struct sockaddr_un remote;
        remote.sun_family = AF_UNIX;
        strncpy(remote.sun_path, out, sizeof(remote.sun_path) - 1);
        if (-1 == connect(s, (struct sockaddr *)&remote, sizeof(struct sockaddr_un))) {
            close(s);
            return;
        }
        write_cwd(s);
        write_call(s, (char const * const *)argv);
        close(s);
    }
}

static char const * concatenate(char const * const argv[]);
static void write_string(int fd, char const * message);

static void write_cwd(int fd) {
    char const * cwd = get_current_dir_name();
    write_string(fd, cwd);
    free((void*)cwd);
}

static void write_call(int fd, char const * const argv[]) {
    char const * cmd = concatenate(argv);
    write_string(fd, cmd);
    free((void *)cmd);
}

static void write_string(int fd, char const * message) {
    size_t const length = strlen(message);
    write(fd, (void const *)&length, sizeof(length));
    if (length > 0) {
        write(fd, (void const *)message, length);
    }
}

static char const * concatenate(char const * const argv[]) {
    char * acc = 0;
    size_t acc_size = 0;

    char const * const * it = argv;
    for (;*it;++it) {
        size_t const sep = (argv == it) ? 0 : 1;
        size_t const it_size = strlen(*it);
        acc = (char *)realloc(acc, acc_size + sep + it_size);
        if (sep) {
            acc[acc_size++] = ' ';
        }
        strncpy((acc + acc_size), *it, it_size);
        acc_size += it_size;
    }
    acc = (char *)realloc(acc, acc_size + 1);
    acc[acc_size++] = '\0';
    return acc;
}

