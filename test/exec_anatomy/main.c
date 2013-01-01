// This file is distributed under MIT-LICENSE. See COPYING for details.

#include <sys/wait.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>

typedef void (*exec_fun)();

void fork_fun(exec_fun f) {
    pid_t child = fork();
    if (-1 == child) {
        perror("fork");
        exit(EXIT_FAILURE);
    } else if (0 == child) {
        (*f)();
    } else {
        int status;
        if (-1 == waitpid(child, &status, 0)) {
            perror("wait");
            exit(EXIT_FAILURE);
        }
    }
}


void call_execv() {
    char * const compiler = "cc";
    char * const argv[] =
        { "cc"
        , "-c"
        , "execv.c"
        , 0
        };

    execv(compiler, argv);
}

void call_execve() {
    char * const compiler = "cc";
    char * const argv[] =
        { "cc"
        , "-c"
        , "execve.c"
        , 0
        };
    char * const envp[] =
        { "THIS=THAT"
        , 0
        };

    execve(compiler, argv, envp);
}

void call_execvp() {
    char * const compiler = "cc";
    char * const argv[] =
        { "cc"
        , "-c"
        , "execvp.c"
        , 0
        };

    execvp(compiler, argv);
}

void call_execvpe() {
    char * const compiler = "cc";
    char * const argv[] =
        { "cc"
        , "-c"
        , "execvpe.c"
        , 0
        };
    char * const envp[] =
        { "THIS=THAT"
        , 0
        };

    execvpe(compiler, argv, envp);
}

void call_execl() {
    char * const compiler = "cc";

    execl(compiler, compiler, "-c", "execl.c", (char *)0);
}

void call_execlp() {
    char * const compiler = "cc";

    execlp(compiler, compiler, "-c", "execlp.c", (char *)0);
}

void call_execle() {
    char * const compiler = "cc";
    char * const envp[] =
        { "THIS=THAT"
        , 0
        };

    execle(compiler, compiler, "-c", "execle.c", (char *)0, envp);
}

int main() {
    fork_fun(call_execv);
    fork_fun(call_execve);
    fork_fun(call_execvp);
    fork_fun(call_execvpe);
    fork_fun(call_execl);
    fork_fun(call_execlp);
    fork_fun(call_execle);
    return 0;
}
