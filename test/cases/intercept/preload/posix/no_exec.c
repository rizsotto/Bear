// REQUIRES: preload
// RUN: %{compile} -o %t %s
// RUN: %{intercept} --verbose --output %t.sqlite3 -- %t
// RUN: assert_intercepted %t.sqlite3 count -ge 1
// RUN: assert_intercepted %t.sqlite3 contains -program %t

#include "config.h"

int main()
{
    return 0;
}
