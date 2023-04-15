// REQUIRES: preload, c_api_execvpe
// RUN: %{compile} '-D_PROGRAM="%{echo}"' -o %t %s
// RUN: %{intercept} --verbose --output %t.json -- %t
// RUN: assert_intercepted %t.json count -eq 2
// RUN: assert_intercepted %t.json contains -program %t -arguments %t
// RUN: assert_intercepted %t.json contains -program %{echo} -arguments %{echo} "hi there"

#include "config.h"

#if defined HAVE_UNISTD_H
#define _GNU_SOURCE
#include <unistd.h>
#endif

int main()
{
    char *const program = _PROGRAM;
    char *const argv[] = { _PROGRAM, "hi there", 0 };
    char *const envp[] = { "THIS=THAT", 0 };
    return execvpe(program, argv, envp);
}
