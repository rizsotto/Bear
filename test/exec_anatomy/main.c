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

#include <sys/wait.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <paths.h>

#if defined HAVE_POSIX_SPAWN || defined HAVE_POSIX_SPAWNP
#include <spawn.h>
#endif

#ifdef NEED_NSGETENVIRON
#include <crt_externs.h>
#else
extern char **environ;
#endif

char ** getenviron()
{
#ifdef NEED_NSGETENVIRON
    return *_NSGetEnviron();
#else
    return environ;
#endif
}


typedef void (*exec_fun)();

void fork_fun(exec_fun f)
{
    pid_t child = fork();
    if (-1 == child)
    {
        perror("fork");
        exit(EXIT_FAILURE);
    }
    else if (0 == child)
    {
        (*f)();
        fprintf(stderr, "children process failed to exec\n");
        exit(EXIT_FAILURE);
    }
    else
    {
        int status;
        if (-1 == waitpid(child, &status, 0))
        {
            perror("wait");
            exit(EXIT_FAILURE);
        }
        int exit_code = WIFEXITED(status) ? WEXITSTATUS(status) : EXIT_FAILURE;
        if (exit_code)
        {
            fprintf(stderr, "children process has non zero exit code\n");
            exit(EXIT_FAILURE);
        }
    }
}


#ifdef HAVE_EXECV
void call_execv()
{
    char * const compiler = "/usr/bin/cc";
    char * const argv[] =
    {
        "cc",
        "-c",
        "execv.c",
        0
    };

    execv(compiler, argv);
}
#endif

#ifdef HAVE_EXECVE
void call_execve()
{
    char * const compiler = "/usr/bin/cc";
    char * const argv[] =
    {
        compiler,
        "-c",
        "execve.c",
        0
    };
    char * const envp[] =
    {
        "THIS=THAT",
        0
    };

    execve(compiler, argv, envp);
}
#endif

#ifdef HAVE_EXECVP
void call_execvp()
{
    char * const compiler = "cc";
    char * const argv[] =
    {
        "cc",
        "-c",
        "execvp.c",
        0
    };

    execvp(compiler, argv);
}
#endif

#ifdef HAVE_EXECVP2
void call_execvP()
{
    char * const compiler = "cc";
    char * const argv[] =
    {
        "cc",
        "-c",
        "execvP.c",
        0
    };

    execvP(compiler, _PATH_DEFPATH, argv);
}
#endif

#ifdef HAVE_EXECVPE
void call_execvpe()
{
    char * const compiler = "cc";
    char * const argv[] =
    {
        "/usr/bin/cc",
        "-c",
        "execvpe.c",
        0
    };
    char * const envp[] =
    {
        "THIS=THAT",
        0
    };

    execvpe(compiler, argv, envp);
}
#endif

#ifdef HAVE_EXECL
void call_execl()
{
    char * const compiler = "/usr/bin/cc";

    execl(compiler, "cc", "-c", "execl.c", (char *)0);
}
#endif

#ifdef HAVE_EXECLP
void call_execlp()
{
    char * const compiler = "cc";

    execlp(compiler, "cc", "-c", "execlp.c", (char *)0);
}
#endif

#ifdef HAVE_EXECLE
void call_execle()
{
    char * const compiler = "/usr/bin/cc";
    char * const envp[] =
    {
        "THIS=THAT",
        0
    };

    execle(compiler, compiler, "-c", "execle.c", (char *)0, envp);
}
#endif

#ifdef HAVE_EXECLE
void call_execle_and_printenv()
{
    char * const envp[] =
    {
        "THIS=THAT",
        0
    };

    char * const pe = "/usr/bin/printenv";
    execle(pe, "printenv", (char *)0, envp);
}
#endif

#ifdef HAVE_POSIX_SPAWN
void call_posix_spawn()
{
    char * const argv[] =
    {
        "cc",
        "-c",
        "posix_spawn.c",
        0
    };

    pid_t child;
    if (0 != posix_spawn(&child, "/usr/bin/cc", 0, 0, argv, getenviron()))
    {
        perror("posix_spawn");
        exit(EXIT_FAILURE);
    }

    int status;
    if (-1 == waitpid(child, &status, 0))
    {
        perror("wait");
        exit(EXIT_FAILURE);
    }
    int exit_code = WIFEXITED(status) ? WEXITSTATUS(status) : EXIT_FAILURE;
    if (exit_code)
    {
        fprintf(stderr, "children process has non zero exit code\n");
        exit(EXIT_FAILURE);
    }
}
#endif

#ifdef HAVE_POSIX_SPAWNP
void call_posix_spawnp()
{
    char * const argv[] =
    {
        "cc",
        "-c",
        "posix_spawnp.c",
        0
    };

    pid_t child;
    if (0 != posix_spawnp(&child, "cc", 0, 0, argv, getenviron()))
    {
        perror("posix_spawnp");
        exit(EXIT_FAILURE);
    }

    int status;
    if (-1 == waitpid(child, &status, 0))
    {
        perror("wait");
        exit(EXIT_FAILURE);
    }
    int exit_code = WIFEXITED(status) ? WEXITSTATUS(status) : EXIT_FAILURE;
    if (exit_code)
    {
        fprintf(stderr, "children process has non zero exit code\n");
        exit(EXIT_FAILURE);
    }
}
#endif

void print_expected_output(FILE *fd, const char *cmd, const char *file, const char *cwd)
{
    static int need_comma = 0;
    if (need_comma)
        fprintf(fd, ",\n");
    fprintf(fd, "{\n");
    fprintf(fd, "  \"directory\": \"%s\",\n", cwd);
    fprintf(fd, "  \"command\": \"%s -c %s\",\n", cmd, file);
    fprintf(fd, "  \"file\": \"%s/%s\"\n", cwd, file);
    fprintf(fd, "}\n");
    need_comma = 1;
}

int main()
{
    char * const cwd = getcwd(NULL, 0);
    FILE *expected_out = fopen("expected.json", "w");
    if (!expected_out)
    {
        perror("fopen");
        exit(EXIT_FAILURE);
    }
    fprintf(expected_out, "[\n");
#ifdef HAVE_EXECV
    print_expected_output(expected_out, "cc", "execv.c", cwd);
    fork_fun(call_execv);
#endif
#ifdef HAVE_EXECVE
    print_expected_output(expected_out, "/usr/bin/cc", "execve.c", cwd);
    fork_fun(call_execve);
#endif
#ifdef HAVE_EXECVP
    print_expected_output(expected_out, "cc", "execvp.c", cwd);
    fork_fun(call_execvp);
#endif
#ifdef HAVE_EXECVP2
    print_expected_output(expected_out, "cc", "execvP.c", cwd);
    fork_fun(call_execvP);
#endif
#ifdef HAVE_EXECVPE
    print_expected_output(expected_out, "/usr/bin/cc", "execvpe.c", cwd);
    fork_fun(call_execvpe);
#endif
#ifdef HAVE_EXECL
    print_expected_output(expected_out, "cc", "execl.c", cwd);
    fork_fun(call_execl);
#endif
#ifdef HAVE_EXECLP
    print_expected_output(expected_out, "cc", "execlp.c", cwd);
    fork_fun(call_execlp);
#endif
#ifdef HAVE_EXECLE
    print_expected_output(expected_out, "/usr/bin/cc", "execle.c", cwd);
    fork_fun(call_execle);
#endif
#ifdef HAVE_POSIX_SPAWN
    print_expected_output(expected_out, "cc", "posix_spawn.c", cwd);
    call_posix_spawn();
#endif
#ifdef HAVE_POSIX_SPAWNP
    print_expected_output(expected_out, "cc", "posix_spawnp.c", cwd);
    call_posix_spawnp();
#endif
#ifdef HAVE_EXECLE
    fork_fun(call_execle_and_printenv);
#endif
    fprintf(expected_out, "]\n");
    fclose(expected_out);
    return 0;
}
