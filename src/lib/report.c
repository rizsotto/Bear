// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "report.h"
#include "../common/stringarray.h"
#include "../common/envarray.h"
#include "../common/protocol.h"

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
static void write_call(int fd, char const * argv[]);

void report_call(char const *method, char * const argv[]) {
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
        write_cwd(s);
        write_call(s, (char const **)argv);
        close(s);
    }
}

static void write_cwd(int fd) {
    char const * cwd = get_current_dir_name();
    write_string(fd, cwd);
    free((void*)cwd);
}

static void write_call(int fd, char const * argv[]) {
    write_string_array(fd, argv);
}

