// This file is distributed under MIT-LICENSE. See COPYING for details.

#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <fcntl.h>
#include <signal.h>


// Stringify environment variables
#define XSTR(s) STR(s)
#define STR(s) #s

static void usage(char const * const name)  __attribute__ ((noreturn));
static void usage(char const * const name) {
    fprintf(stderr, "Usage: %s [-o output] [-b libear] [-d socket]-- command\n", name);
    exit(EXIT_FAILURE);
}

static pid_t    child_pid;
static int      child_status = EXIT_FAILURE;

static sigset_t signal_mask;


static void handler(int signum) {
    switch (signum) {
    case SIGCHLD: {
        int status;
        while (0 > waitpid(WAIT_ANY, &status, WNOHANG)) ;
        child_status = WIFEXITED(status) ? WEXITSTATUS(status) : EXIT_FAILURE;
        break;
        }
    case SIGINT:
        kill(child_pid, signum);
    default:
        break;
    }
}

static void copy(int in, int out) {
    size_t const buffer_size = 1024;
    char buffer[buffer_size];
    for (;;) {
        int current_read = read(in, &buffer, buffer_size);
        if (-1 == current_read) {
            perror("read");
            exit(EXIT_FAILURE);
        }
        int current_write = write(out, &buffer, current_read);
        if (-1 == current_write) {
            perror("write");
            exit(EXIT_FAILURE);
        }
        if (buffer_size > current_read) {
            break;
        }
    }
}

static int collect(char const * socket_file, char const * output_file) {
    // open the output file
    int output_fd = open(output_file, O_CREAT|O_APPEND|O_RDWR, S_IRUSR|S_IWUSR);
    if (-1 == output_fd) {
        perror("open");
        exit(EXIT_FAILURE);
    }
    // set up socket
    struct sockaddr_un local;
    int listen_sock = socket(AF_UNIX, SOCK_STREAM, 0);
    if (-1 == listen_sock) {
        perror("socket");
        exit(EXIT_FAILURE);
    }
    memset(&local, 0, sizeof(struct sockaddr_un));
    local.sun_family = AF_UNIX;
    strncpy(local.sun_path, socket_file, sizeof(local.sun_path) - 1);
    if ((-1 == unlink(socket_file)) && (ENOENT != errno)) {
        perror("unlink");
        exit(EXIT_FAILURE);
    }
    if (-1 == bind(listen_sock, (struct sockaddr *)&local, sizeof(struct sockaddr_un))) {
        perror("bind");
        exit(EXIT_FAILURE);
    }
    if (-1 == listen(listen_sock, 0)) {
        perror("listen");
        exit(EXIT_FAILURE);
    }
    // enable signals for accept loop
    if (-1 == sigprocmask(SIG_UNBLOCK, &signal_mask, 0)) {
        perror("sigprocmask");
        exit(EXIT_FAILURE);
    }
    // do the job
    int conn_sock;
    while ((conn_sock = accept(listen_sock, 0, 0)) != -1) {
        copy(conn_sock, output_fd);
        close(conn_sock);
    }
    // skip errors during shutdown
    close(output_fd);
    close(listen_sock);
    unlink(socket_file);
    return child_status;
}

int main(int argc, char * const argv[]) {
    char const * socket_file = "/tmp/bear.socket";
    char const * libear_path = XSTR(LIBEAR_INSTALL_DIR);
    char const * output_file = 0;
    char * const * unprocessed_argv = 0;
    // parse command line arguments.
    int flags, opt;
    while ((opt = getopt(argc, argv, "o:b:d:")) != -1) {
        switch (opt) {
        case 'o':
            output_file = optarg;
            break;
        case 'b':
            libear_path = optarg;
            break;
        case 'd':
            socket_file = optarg;
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
    child_pid = fork();
    if (-1 == child_pid) {
        perror("fork");
        exit(EXIT_FAILURE);
    }
    if (0 == child_pid) {
        // child process
        if (-1 == setenv("LD_PRELOAD", libear_path, 1)) {
            perror("setenv");
            exit(EXIT_FAILURE);
        }
        if (-1 == setenv("BEAR_OUTPUT", socket_file, 1)) {
            perror("setenv");
            exit(EXIT_FAILURE);
        }
        if (-1 == execvp(*unprocessed_argv, unprocessed_argv)) {
            perror("execvp");
            exit(EXIT_FAILURE);
        }
    } else {
        // parent process
        // block all signals until we reach the blocking accept
        sigfillset(&signal_mask);
        if (-1 == sigprocmask(SIG_BLOCK, &signal_mask, 0)) {
            perror("sigprocmask");
            exit(EXIT_FAILURE);
        }
        // install signal handlers
        struct sigaction action, old_action;
        action.sa_mask = signal_mask;
        action.sa_handler = handler;
        action.sa_flags = 0;
        if (-1 == sigaction(SIGCHLD,&action,&old_action)) {
            perror( "sigaction");
            exit(EXIT_FAILURE);
        }
        if (-1 == sigaction(SIGINT,&action,&old_action)) {
            perror( "sigaction");
            exit(EXIT_FAILURE);
        }
        // go for the data
        return collect(socket_file, output_file);
    }
    // never gets here
    return 0;
}

