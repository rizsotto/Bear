// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "config.h"

#include <sys/wait.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <paths.h>

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
#ifdef HAVE_EXECLE
    fork_fun(call_execle_and_printenv);
#endif
    fprintf(expected_out, "]\n");
    fclose(expected_out);
    return 0;
}
