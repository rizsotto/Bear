// REQUIRES: preload, c_api_posix_spawn
// RUN: cd %T; %{compile} '-D_PROGRAM="%{echo}"' -o %t %s
// RUN: %{intercept} --verbose --output %t.json -- %t
// RUN: assert_intercepted %t.json count -eq 2
// RUN: assert_intercepted %t.json contains -program %t -arguments %t
// RUN: assert_intercepted %t.json contains -program %{echo} -arguments %{echo} "hi there"

#include "config.h"

#include <stdio.h>
#include <stdlib.h>
#include <errno.h>

#if defined HAVE_SPAWN_H
#include <spawn.h>
#endif

#if defined HAVE_SYS_TYPES_H
#include <sys/types.h>
#endif
#if defined HAVE_SYS_WAIT_H
#include <sys/wait.h>
#endif

int main()
{
    char *const program = _PROGRAM;
    char *const argv[] = { _PROGRAM, "hi there", 0 };
    char *const envp[] = { "THIS=THAT", 0 };

    pid_t child;
    if (0 != posix_spawn(&child, program, 0, 0, argv, envp)) {
        perror("posix_spawn");
        exit(EXIT_FAILURE);
    }

    int status;
    if (-1 == waitpid(child, &status, 0)) {
        perror("wait");
        return EXIT_FAILURE;
    }
    if (WIFEXITED(status) ? WEXITSTATUS(status) : EXIT_FAILURE) {
        fprintf(stderr, "child process has non zero exit code\n");
        return EXIT_FAILURE;
    }
    return EXIT_SUCCESS;
}
