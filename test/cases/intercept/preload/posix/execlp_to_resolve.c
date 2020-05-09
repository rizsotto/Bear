// REQUIRES: preload, c_api_execlp
// RUN: cd %T; %{compile} '-D_PROGRAM="echo"' -o %t %s
// RUN: %{intercept} --verbose --output %t.json -- %t
// RUN: assert_intercepted %t.json count -eq 2
// RUN: assert_intercepted %t.json contains -program %t -arguments %t
// RUN: assert_intercepted %t.json contains -program %{echo} -arguments echo "hi there"

#include "config.h"
#include <unistd.h>

int main()
{
    char *const program = _PROGRAM;
    return execlp(program, program, "hi there", 0);
}
