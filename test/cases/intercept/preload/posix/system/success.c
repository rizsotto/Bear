// REQUIRES: preload, c_api_system
// RUN: %{compile} -o %t %s
// RUN: %{intercept} --verbose --output %t.events.db -- %t
// RUN: %{events_db} dump --path %t.events.db --output %t.json
// RUN: assert_intercepted %t.json count -ge 2
// RUN: assert_intercepted %t.json contains -program %t
// RUN: assert_intercepted %t.json contains -arguments ls -l

#include <stdlib.h>

int main () {
    char *const command = "ls -l";

    system(command);

    return EXIT_SUCCESS;
}
