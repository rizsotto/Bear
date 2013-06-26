/*  Copyright (C) 2012, 2013 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#include "config.h"
#include "protocol.h"
#include "output.h"

#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include <string.h>
#include <sys/wait.h>
#include <signal.h>

// variables which are used in signal handler
static volatile pid_t    child_pid;
static volatile int      child_status = EXIT_FAILURE;

static void usage(char const * const name)  __attribute__ ((noreturn));
static void mask_all_signals(int command);
static void install_signal_handler(int signum);
static void collect_messages(char const * socket, char const * output, int debug, int sync_fd);
static void notify_child(int fd);
static void wait_for_parent(int fd);

int main(int argc, char * const argv[])
{
    char const * output_file = DEFAULT_OUTPUT_FILE;
    char const * libear_path = DEFAULT_PRELOAD_FILE;
    char * socket_file = 0;
    char const * socket_dir = 0;
    int debug = 0;
    char * const * unprocessed_argv = 0;
    int sync_fd[2];
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
    if (0 == socket_file)
    {
        char template[] = "/tmp/bear-XXXXXX";
        socket_dir = mkdtemp(template);
        if (0 == socket_dir)
        {
            perror("bear: mkdtemp");
            exit(EXIT_FAILURE);
        }
        if (-1 == asprintf(&socket_file, "%s/socket", socket_dir))
        {
            perror("bear: asprintf");
            exit(EXIT_FAILURE);
        }
    }
    // set up sync pipe
    if (-1 == pipe(sync_fd))
    {
        perror("bear: pipe");
        exit(EXIT_FAILURE);
    }
    // fork
    child_pid = fork();
    if (-1 == child_pid)
    {
        perror("bear: fork");
        exit(EXIT_FAILURE);
    }
    else if (0 == child_pid)
    {
        // child process
        close(sync_fd[1]);
        wait_for_parent(sync_fd[0]);
        if (-1 == setenv(ENV_PRELOAD, libear_path, 1))
        {
            perror("bear: setenv");
            exit(EXIT_FAILURE);
        }
        if (-1 == setenv(ENV_OUTPUT, socket_file, 1))
        {
            perror("bear: setenv");
            exit(EXIT_FAILURE);
        }
#ifdef ENV_FLAT
        if (-1 == setenv(ENV_FLAT, "1", 1))
        {
            perror("bear: setenv");
            exit(EXIT_FAILURE);
        }
#endif
        if (-1 == execvp(*unprocessed_argv, unprocessed_argv))
        {
            perror("bear: execvp");
            exit(EXIT_FAILURE);
        }
    }
    else
    {
        // parent process
        install_signal_handler(SIGCHLD);
        install_signal_handler(SIGINT);
        mask_all_signals(SIG_BLOCK);
        close(sync_fd[0]);
        collect_messages(socket_file, output_file, debug, sync_fd[1]);
        if (socket_dir)
        {
            rmdir(socket_dir);
            free((void *)socket_file);
        }
    }
    return child_status;
}

static void receive_on_unix_socket(char const * socket_file, int output_fd, int debug, int sync_fd);

static void collect_messages(char const * socket_file, char const * output_file, int debug, int sync_fd)
{
    // open the output file
    int output_fd = bear_open_json_output(output_file);
    // remove old socket file if any
    if ((-1 == unlink(socket_file)) && (ENOENT != errno))
    {
        perror("bear: unlink");
        exit(EXIT_FAILURE);
    }
    // receive messages
    receive_on_unix_socket(socket_file, output_fd, debug, sync_fd);
    // skip errors during shutdown
    bear_close_json_output(output_fd);
    unlink(socket_file);
}

static void receive_on_unix_socket(char const * file, int out_fd, int debug, int sync_fd)
{
    int s = bear_create_unix_socket(file);
    mask_all_signals(SIG_UNBLOCK);
    notify_child(sync_fd);
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
        perror("bear: sigemptyset");
        exit(EXIT_FAILURE);
    }
    if (0 != sigaddset(&action.sa_mask, signum))
    {
        perror("bear: sigaddset");
        exit(EXIT_FAILURE);
    }
    if (0 != sigaction(signum, &action, NULL))
    {
        perror("bear: sigaction");
        exit(EXIT_FAILURE);
    }
}

static void mask_all_signals(int command)
{
    sigset_t signal_mask;
    if (0 != sigfillset(&signal_mask))
    {
        perror("bear: sigfillset");
        exit(EXIT_FAILURE);
    }
    if (0 != sigprocmask(command, &signal_mask, 0))
    {
        perror("bear: sigprocmask");
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
            DEFAULT_OUTPUT_FILE,
            DEFAULT_PRELOAD_FILE);
    exit(EXIT_FAILURE);
}

static void notify_child(int fd)
{
    if (-1 == write(fd, "ready", 5))
    {
        perror("bear: write");
        exit(EXIT_FAILURE);
    }
    close(fd);
}

static void wait_for_parent(int fd)
{
    char buffer[5];
    if (-1 == read(fd, buffer, sizeof(buffer)))
    {
        perror("bear: read");
        exit(EXIT_FAILURE);
    }
    close(fd);
}
