// REQUIRES: preload, c_api_execle
// RUN: %{compile} '-D_PROGRAM="%{echo}"' -o %t %s
// RUN: %{intercept} --verbose --output %t.sqlite3 -- %t
// RUN: %{events_db} dump --path %t.sqlite3 --output %t.json
// RUN: assert_intercepted %t.json count -eq 2
// RUN: assert_intercepted %t.json contains -program %t -arguments %t
// RUN: assert_intercepted %t.json contains -program %{echo} -arguments %{echo} "hi there"

#include "config.h"

#if defined HAVE_UNISTD_H
#include <unistd.h>
#endif

int main()
{
    char *const program = _PROGRAM;
    char *const envp[] = { "THIS=THAT", 0 };
    return execle(program, program, "hi there", 0, envp);
}
