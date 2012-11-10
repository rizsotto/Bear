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

#define SOCKET_FILE XSTR(DEFAULT_SOCKET_FILE)
#define OUTPUT_FILE XSTR(DEFAULT_OUTPUT_FILE)
#define LIBEAR_FILE XSTR(LIBEAR_INSTALL_DIR)"/libear.so"

// variables which are used in signal handler
static pid_t    child_pid;
static int      child_status = EXIT_FAILURE;


static void usage(char const * const name)  __attribute__ ((noreturn));
static void mask_all_signals(int command);
static void install_signal_handler(int signum);
static void collect(char const * socket_file, char const * output_file);


int main(int argc, char * const argv[]) {
    char const * socket_file = SOCKET_FILE;
    char const * output_file = OUTPUT_FILE;
    char const * libear_path = LIBEAR_FILE;
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
    // validate
    if (argc == optind) {
        usage(argv[0]);
    }
    unprocessed_argv = &(argv[optind]);
    // fork
    child_pid = fork();
    if (-1 == child_pid) {
        perror("fork");
        exit(EXIT_FAILURE);
    } else if (0 == child_pid) {
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
        install_signal_handler(SIGCHLD);
        install_signal_handler(SIGINT);
        // go for the data
        collect(socket_file, output_file);
    }
    return child_status;
}

static void copy(int in, int out);

static void collect(char const * socket_file, char const * output_file) {
    mask_all_signals(SIG_BLOCK);
    // open the output file
    int output_fd = open(output_file, O_CREAT|O_RDWR, S_IRUSR|S_IWUSR);
    if (-1 == output_fd) {
        perror("open");
        exit(EXIT_FAILURE);
    }
    // remove old socket file if any
    if ((-1 == unlink(socket_file)) && (ENOENT != errno)) {
        perror("unlink");
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
    if (-1 == bind(listen_sock, (struct sockaddr *)&local, sizeof(struct sockaddr_un))) {
        perror("bind");
        exit(EXIT_FAILURE);
    }
    if (-1 == listen(listen_sock, 0)) {
        perror("listen");
        exit(EXIT_FAILURE);
    }
    // do the job
    mask_all_signals(SIG_UNBLOCK);
    int conn_sock;
    while ((conn_sock = accept(listen_sock, 0, 0)) != -1) {
        mask_all_signals(SIG_BLOCK);
        copy(conn_sock, output_fd);
        close(conn_sock);
        mask_all_signals(SIG_UNBLOCK);
    }
    // skip errors during shutdown
    close(output_fd);
    close(listen_sock);
    unlink(socket_file);
}

static char const * read_string(int in) {
    size_t length = 0;
    if (-1 == read(in, (void *)&length, sizeof(size_t))) {
        perror("read: header");
        exit(EXIT_FAILURE);
    }
    if (length > 0) {
        char * result = malloc(length + 1);
        if (-1 == read(in, (void *)result, length)) {
            free(result);
            perror("read: message");
            exit(EXIT_FAILURE);
        }
        result[length] = '\0';
        return result;
    }
    return "";
}

static void copy(int in, int out) {
    char const * const cwd = read_string(in);
    char const * const cmd = read_string(in);
    // FIXME: do filtering and formating
    write(out, cwd, strlen(cwd));
    write(out, "\n", 1);
    write(out, cmd, strlen(cmd));
    write(out, "\n", 1);
    free((void *)cwd);
    free((void *)cmd);
}

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

static void install_signal_handler(int signum) {
    struct sigaction action, old_action;
    sigemptyset(&action.sa_mask);
    action.sa_handler = handler;
    action.sa_flags = 0;
    if (-1 == sigaction(signum, &action, &old_action)) {
        perror( "sigaction");
        exit(EXIT_FAILURE);
    }
}

static void mask_all_signals(int command) {
    sigset_t signal_mask;
    sigfillset(&signal_mask);
    if (-1 == sigprocmask(command, &signal_mask, 0)) {
        perror("sigprocmask");
        exit(EXIT_FAILURE);
    }
}

static void usage(char const * const name) {
    fprintf(stderr,
            "Usage: %s [-o output] [-b libear] [-d socket] -- command\n"
            "\n"
            "   -o output   output file (default: %s)\n"
            "   -b libear   libear.so location (default: %s)\n"
            "   -d socket   multiplexing socket (default: %s)\n",
            name,
            OUTPUT_FILE,
            LIBEAR_FILE,
            SOCKET_FILE);
    exit(EXIT_FAILURE);
}

