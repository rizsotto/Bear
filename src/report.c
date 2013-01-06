// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "environ.h"
#include "protocol.h"

#include <unistd.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <sys/un.h>

typedef void (*send_message)(char const *, struct bear_message const *);

static void report(send_message fp, char const * out_file, char const * fun, char * const argv[]) {
    struct bear_message msg;
    {
        msg.pid = getpid();
        msg.fun = fun;
        msg.cwd = get_current_dir_name();
        msg.cmd = (char const **)argv;
    }
    (*fp)(out_file, &msg);
    free((void*)msg.cwd);
}

static void send_on_unix_socket(char const * out, struct bear_message const * msg) {
    int s = socket(AF_UNIX, SOCK_STREAM, 0);
    if (-1 == s) {
        perror("socket");
        exit(EXIT_FAILURE);
    }
    struct sockaddr_un remote;
    remote.sun_family = AF_UNIX;
    strncpy(remote.sun_path, out, sizeof(remote.sun_path) - 1);
    if (-1 == connect(s, (const struct sockaddr *)&remote, sizeof(struct sockaddr_un))) {
        perror("connect");
        exit(EXIT_FAILURE);
    }
    bear_write_message(s, msg);
    close(s);
}

void report_call(char const *fun, char * const argv[]) {
    char * const out = getenv(ENV_OUTPUT);
    if (0 == out) {
        perror("getenv");
        exit(EXIT_FAILURE);
    }

    return report(send_on_unix_socket, out, fun, argv);
}
