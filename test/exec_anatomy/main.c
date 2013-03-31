// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "config.h"

#include <sys/wait.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>

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
    }
    else
    {
        int status;
        if (-1 == waitpid(child, &status, 0))
        {
            perror("wait");
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

int main()
{
#ifdef HAVE_EXECV
    fork_fun(call_execv);
#endif
#ifdef HAVE_EXECVE
    fork_fun(call_execve);
#endif
#ifdef HAVE_EXECVP
    fork_fun(call_execvp);
#endif
#ifdef HAVE_EXECVPE
    fork_fun(call_execvpe);
#endif
#ifdef HAVE_EXECL
    fork_fun(call_execl);
#endif
#ifdef HAVE_EXECLP
    fork_fun(call_execlp);
#endif
#ifdef HAVE_EXECLE
    fork_fun(call_execle);
#endif
#ifdef HAVE_EXECLE
    fork_fun(call_execle_and_printenv);
#endif
    return 0;
}
