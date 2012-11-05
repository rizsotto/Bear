// This file is distributed under MIT-LICENSE. See COPYING for details.

#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <sys/stat.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/signalfd.h>
#include <sys/epoll.h>
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

static void copy(int in, int out);
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

static int setup_sigchld_fd();
static int setup_sigchld_fd() {
    int result;
    sigset_t mask;
    sigfillset(&mask);
    sigdelset(&mask, SIGINT);

    result = signalfd(-1, &mask, 0);
    if (-1 == result) {
        perror("signalfd");
        exit(EXIT_FAILURE);
    }
    fcntl(result, F_SETFL, fcntl(result, F_GETFL, 0) | O_NONBLOCK);
    return result;
}

static int setup_socket(char const * socket_file);
static int setup_socket(char const * socket_file) {
    struct sockaddr_un local;
    int socket_fd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (-1 == socket_fd) {
        perror("socket");
        exit(EXIT_FAILURE);
    }
    memset(&local, 0, sizeof(struct sockaddr_un));
    local.sun_family = AF_UNIX;
    strncpy(local.sun_path, socket_file, sizeof(local.sun_path) - 1);
    if (-1 == bind(socket_fd, (struct sockaddr *)&local, sizeof(struct sockaddr_un))) {
        perror("bind");
        exit(EXIT_FAILURE);
    }
    if (-1 == listen(socket_fd, 0)) {
        perror("listen");
        exit(EXIT_FAILURE);
    }
    return socket_fd;
}

static int loop(int listen_sock, int sigchld_fd, int output_fd);
static int loop(int listen_sock, int sigchld_fd, int output_fd) {
    #define MAX_EVENTS 10
    struct epoll_event ev, events[MAX_EVENTS];
    int conn_sock, nfds, epollfd;
    int n;

    sigset_t mask;
    sigfillset(&mask);
    sigdelset(&mask, SIGINT);

    epollfd = epoll_create(2);
    if (-1 == epollfd) {
        perror("epoll_create");
        exit(EXIT_FAILURE);
    }

    memset(&ev, 0, sizeof(ev));
    ev.events = EPOLLIN;
    ev.data.fd = listen_sock;
    if (-1 == epoll_ctl(epollfd, EPOLL_CTL_ADD, listen_sock, &ev)) {
        perror("epoll_ctl: listen_sock");
        exit(EXIT_FAILURE);
    }

    memset(&ev, 0, sizeof(ev));
    ev.events = EPOLLIN;
    ev.data.fd = sigchld_fd;
    if (-1 == epoll_ctl(epollfd, EPOLL_CTL_ADD, sigchld_fd, &ev)) {
        perror("epoll_ctl: sigchld_fd");
        exit(EXIT_FAILURE);
    }

    for (;;) {
        nfds = epoll_pwait(epollfd, events, MAX_EVENTS, -1, &mask);
        if (-1 == nfds) {
            perror("epoll_pwait");
            exit(EXIT_FAILURE);
        }

        for (n = 0; n < nfds; ++n) {
            if (events[n].data.fd == listen_sock) {
                int conn_sock = accept(listen_sock, 0, 0);
                if (-1 == conn_sock) {
                    perror("accept");
                    exit(EXIT_FAILURE);
                }
                copy(conn_sock, output_fd);
                close(conn_sock);
            } else if (events[n].data.fd == sigchld_fd) {
                int status;
                pid_t pid = waitpid(-1, &status, WNOHANG);
                return WIFEXITED(status) ? WEXITSTATUS(status) : 0;
            }
        }
    }
}

static int collect_and_dump(char const * socket_file, char const * output_file);
static int collect_and_dump(char const * socket_file, char const * output_file) {
    unlink(socket_file);
    int socket_fd = setup_socket(socket_file);
    int sigchld_fd = setup_sigchld_fd();
    int out_fd = open(output_file, O_CREAT|O_APPEND|O_RDWR, S_IRUSR|S_IWUSR);
    // call dispatch method
    int result = loop(socket_fd, sigchld_fd, out_fd);
    // shutdown
    close(out_fd);
    close(socket_fd);
    close(sigchld_fd);
    unlink(socket_file);
    return result;
}

int main(int argc, char * const argv[]);
int main(int argc, char * const argv[]) {
    char const * socket_file = "/tmp/bear.socket";
    char const * libear_path = XSTR(LIBEAR_INSTALL_DIR);
    char const * output_file = 0;
    char * const * unprocessed_argv = 0;
    pid_t pid;
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
    pid = fork();
    if (-1 == pid) {
        perror("fork");
        exit(EXIT_FAILURE);
    }
    if (0 == pid) {
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
        return collect_and_dump(socket_file, output_file);
    }
    // never gets here
    return 0;
}

