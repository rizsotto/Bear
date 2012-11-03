// This file is distributed under MIT-LICENSE. See COPYING for details.

#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <sys/types.h>
#include <sys/wait.h>


// Stringify environment variables
#define XSTR(s) STR(s)
#define STR(s) #s


static void usage(char const * const name)  __attribute__ ((noreturn));
static void usage(char const * const name) {
    fprintf(stderr, "Usage: %s [-o output] [-b libear] -- command\n", name);
    exit(EXIT_FAILURE);
}

static void runtime_error(char const * const msg) __attribute__ ((noreturn));
static void runtime_error(char const * const msg) {
    fprintf(stderr, "%s, error %d\n", msg, errno);
    exit(EXIT_FAILURE);
}

int main(int argc, char * const argv[]);
int main(int argc, char * const argv[]) {
    char const * libear_path = XSTR(LIBEAR_INSTALL_DIR);
    char const * output_file = 0;
    char * const * unprocessed_argv = 0;
    pid_t pid;
    int status;
    // parse command line arguments.
    int flags, opt;
    while ((opt = getopt(argc, argv, "o:b:")) != -1) {
        switch (opt) {
        case 'o':
            output_file = optarg;
            break;
        case 'b':
            libear_path = optarg;
            break;
        default: /* '?' */
            usage(argv[0]);
        }
    }
    if ((argc == optind) || (0 == output_file)) {
        usage(argv[0]);
    }
    unprocessed_argv = &(argv[optind]);
    // fork
    pid = fork();
    if (-1 == pid) {
        runtime_error("can't fork");
    }
    if (0 == pid) {
        // child process
        if (-1 == setenv("LD_PRELOAD", libear_path, 1)) {
            runtime_error("can't setenv");
        }
        if (-1 == setenv("BEAR_OUTPUT", output_file, 1)) {
            runtime_error("can't setenv");
        }
        if (-1 == execvp(*unprocessed_argv, unprocessed_argv)) {
            runtime_error("can't execvp");
        }
    } else {
        // parent process
        if (-1 == wait(&status)) {
            runtime_error("can't wait");
        }
    }
    return status;
}

