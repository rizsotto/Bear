// REQUIRES: preload, c_api_posix_spawnp
// RUN: cd %T; %{compile} '-D_PROGRAM="/path/to/not/existing"' -o %t %s
// RUN: %t > %t.without.errno
// RUN: %{intercept} --output %t.json -- %t > %t.with.errno
// RUN: diff %t.with.errno %t.without.errno
// RUN: assert_intercepted %t.json count -eq 1
// RUN: assert_intercepted %t.json contains -program %t

#include "config.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined HAVE_SPAWN_H
#include <spawn.h>
#endif

extern char **environ;

int main()
{
    char *const program = _PROGRAM;
    char *const argv[] = { _PROGRAM, "hi there", 0 };

    pid_t child;
    if (0 != posix_spawnp(&child, program, 0, 0, argv, environ)) {
        const int error = errno;
        printf("errno: %d (%s)\n", error, strerror(error));
        return EXIT_SUCCESS;
    }

    return EXIT_FAILURE;
}
