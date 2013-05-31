// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "protocol.h"
#include "output.h"
#include "environ.h"

#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include <string.h>
#include <sys/wait.h>
#include <signal.h>

#define OUTPUT_FILE DEFAULT_OUTPUT_FILE
#define LIBEAR_FILE DEFAULT_PRELOAD_FILE

// variables which are used in signal handler
static volatile pid_t    child_pid;
static volatile int      child_status = EXIT_FAILURE;

static void usage(char const * const name)  __attribute__ ((noreturn));
static void mask_all_signals(int command);
static void install_signal_handler(int signum);
static void collect_messages(char const * socket, char const * output, int debug);

#define SOCKET_DIRECTORY DEFAULT_TEMP_DIRECTORY "/bear.XXXXXX";

int main(int argc, char * const argv[])
{
    char const * socket_file = NULL;
    char const * output_file = OUTPUT_FILE;
    char const * libear_path = LIBEAR_FILE;
    char socket_directory[] = SOCKET_DIRECTORY;
    char temp_socket[] = SOCKET_DIRECTORY "/socket";
    int debug = 0;
    char * const * unprocessed_argv = 0;
    int use_temp_socket = 0;
    // parse command line arguments.
    int opt;
    while ((opt = getopt(argc, argv, "o:b:s:dceh?")) != -1)
    {
        switch (opt)
        {
        case 'o':
            output_file = optarg;
            break;
        case 'b':
            libear_path = optarg;
            break;
        case 's':
            socket_file = optarg;
            break;
        case 'd':
            debug = 1;
            break;
        case 'c':
            bear_print_known_compilers();
            return 0;
        case 'e':
            bear_print_known_extensions();
            return 0;
        case 'h':
        default: /* '?' */
            usage(argv[0]);
        }
    }
    // validate
    if (argc == optind)
    {
        usage(argv[0]);
    }
    unprocessed_argv = &(argv[optind]);
    // create temporary directory for socket
    if (NULL == socket_file)
    {
        if (NULL == mkdtemp(socket_directory))
        {
            perror("mkdtemp");
            exit(EXIT_FAILURE);
        }

        // replace XXXXXX in temp_socket with the new content from socket_directory
        strncpy(temp_socket, socket_directory, sizeof(socket_directory) - 1);
        socket_file = temp_socket;
        use_temp_socket = 1;
    }
    // fork
    child_pid = fork();
    if (-1 == child_pid)
    {
        perror("fork");
        exit(EXIT_FAILURE);
    }
    else if (0 == child_pid)
    {
        // child process
        if (-1 == setenv(ENV_PRELOAD, libear_path, 1))
        {
            perror("setenv");
            exit(EXIT_FAILURE);
        }
        if (-1 == setenv(ENV_OUTPUT, socket_file, 1))
        {
            perror("setenv");
            exit(EXIT_FAILURE);
        }
#ifdef ENV_FLAT
        if (-1 == setenv(ENV_FLAT, "1", 1))
        {
            perror("setenv");
            exit(EXIT_FAILURE);
        }
#endif
        if (-1 == execvp(*unprocessed_argv, unprocessed_argv))
        {
            perror("execvp");
            exit(EXIT_FAILURE);
        }
    }
    else
    {
        // parent process
        install_signal_handler(SIGCHLD);
        install_signal_handler(SIGINT);
        mask_all_signals(SIG_BLOCK);
        collect_messages(socket_file, output_file, debug);
        if (1 == use_temp_socket)
        {
            // remove temporary directory
            rmdir(socket_directory);
        }
    }
    return child_status;
}

static void receive_on_unix_socket(char const * socket_file, int output_fd, int debug);

static void collect_messages(char const * socket_file, char const * output_file, int debug)
{
    // open the output file
    int output_fd = bear_open_json_output(output_file);
    // remove old socket file if any
    if ((-1 == unlink(socket_file)) && (ENOENT != errno))
    {
        perror("unlink");
        exit(EXIT_FAILURE);
    }
    // receive messages
    receive_on_unix_socket(socket_file, output_fd, debug);
    // skip errors during shutdown
    bear_close_json_output(output_fd);
    unlink(socket_file);
}

static void receive_on_unix_socket(char const * file, int out_fd, int debug)
{
    int s = bear_create_unix_socket(file);
    mask_all_signals(SIG_UNBLOCK);
    struct bear_message msg;
    while ((child_pid) && bear_accept_message(s, &msg))
    {
        mask_all_signals(SIG_BLOCK);
        bear_append_json_output(out_fd, &msg, debug);
        bear_free_message(&msg);
        mask_all_signals(SIG_UNBLOCK);
    }
    mask_all_signals(SIG_BLOCK);
    close(s);
}

static void handler(int signum)
{
    switch (signum)
    {
    case SIGCHLD:
    {
        int status;
        while (0 > waitpid(WAIT_ANY, &status, WNOHANG)) ;
        child_status = WIFEXITED(status) ? WEXITSTATUS(status) : EXIT_FAILURE;
        child_pid = 0;
        break;
    }
    case SIGINT:
        kill(child_pid, signum);
    default:
        break;
    }
}

static void install_signal_handler(int signum)
{
    struct sigaction action;
    action.sa_handler = handler;
    action.sa_flags = 0;
    if (0 != sigemptyset(&action.sa_mask))
    {
        perror( "sigemptyset");
        exit(EXIT_FAILURE);
    }
    if (0 != sigaddset(&action.sa_mask, signum))
    {
        perror( "sigaddset");
        exit(EXIT_FAILURE);
    }
    if (0 != sigaction(signum, &action, NULL))
    {
        perror( "sigaction");
        exit(EXIT_FAILURE);
    }
}

static void mask_all_signals(int command)
{
    sigset_t signal_mask;
    if (0 != sigfillset(&signal_mask))
    {
        perror("sigfillset");
        exit(EXIT_FAILURE);
    }
    if (0 != sigprocmask(command, &signal_mask, 0))
    {
        perror("sigprocmask");
        exit(EXIT_FAILURE);
    }
}

static void usage(char const * const name)
{
    fprintf(stderr,
            "Usage: %s [-o output] [-b libear] [-d socket] -- command\n"
            "\n"
            "   -o output   output file (default: %s)\n"
            "   -b libear   library location (default: %s)\n"
            "   -s socket   multiplexing socket (default: randomly generated)\n"
            "   -d          debug output (default: disabled)\n"
            "   -c          print out known compilers\n"
            "   -e          print out known source file extensions\n"
            "   -h          this message\n",
            name,
            OUTPUT_FILE,
            LIBEAR_FILE);
    exit(EXIT_FAILURE);
}

