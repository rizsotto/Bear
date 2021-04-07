// REQUIRES: preload
// RUN: %{compile} -o %t %s
// RUN: %{intercept} --verbose --output %t.events.db -- %t
// RUN: %{events_db} dump --path %t.events.db --output %t.json
// RUN: assert_intercepted %t.json count -ge 1
// RUN: assert_intercepted %t.json contains -program %t

#include "config.h"

int main()
{
    return 0;
}
