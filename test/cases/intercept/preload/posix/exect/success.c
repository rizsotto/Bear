// REQUIRES: preload, c_api_exect
// RUN: %{compile} '-D_PROGRAM="%{echo}"' -o %t %s
// RUN: %{intercept} --verbose --output %t.sqlite3 -- %t
// RUN: assert_intercepted %t.sqlite3 count -eq 2
// RUN: assert_intercepted %t.sqlite3 contains -program %t -arguments %t
// RUN: assert_intercepted %t.sqlite3 contains -program %{echo} -arguments %{echo} "hi there"

#include "config.h"

#if defined HAVE_UNISTD_H
#include <unistd.h>
#endif

int main()
{
    char *const program = _PROGRAM;
    char *const argv[] = { _PROGRAM, "hi there", 0 };
    char *const envp[] = { "THIS=THAT", 0 };
    return exect(program, argv, envp);
}
