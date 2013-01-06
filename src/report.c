// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "report.h"
#include "stringarray.h"
#include "envarray.h"
#include "protocol.h"

#include <unistd.h>
#include <malloc.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <sys/un.h>

static void write_message(int fd, char const * function, char * const argv[]);

void report_call(char const *function, char * const argv[]) {
    // get output file name
    char * const out = getenv(ENV_OUTPUT);
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
        write_message(s, function, argv);
        close(s);
    }
}

static void write_message(int fd, char const * function, char * const argv[]) {
    struct bear_message msg;
    {
        msg.pid = getpid();
        msg.fun = function;
        msg.cwd = get_current_dir_name();
        msg.cmd = argv;
    }
    bear_write_message(fd, &msg);
    free((void*)msg.cwd);
}
