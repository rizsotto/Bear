// REQUIRES: preload
// RUN: %{compile} -o %t %s
// RUN: %{intercept} --verbose --output %t.json -- %t
// RUN: assert_intercepted %t.json count -ge 1
// RUN: assert_intercepted %t.json contains -program %t

#include "config.h"

int main()
{
    return 0;
}
