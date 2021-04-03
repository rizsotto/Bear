// REQUIRES: preload, shell, dynamic-shell
// RUN: %{compile} '-D_FILE="%t.state"' -fpic -shared -o %t.so %s
// RUN: env LD_PRELOAD=%t.so %{intercept} --verbose --output %t.sqlite3 -- %{shell} -c %{true}
// RUN: %{events_db} dump --path %t.sqlite3 --output %t.json
// RUN: assert_intercepted %t.json count -ge 2
// RUN: assert_intercepted %t.json contains -arguments %{shell} -c %{true}
// RUN: assert_intercepted %t.json contains -program %{true} -arguments %{true}
// RUN: test -f %t.state

#include <stdio.h>

void on_load() __attribute__((constructor));
void on_load()
{
    const char* file = _FILE;

    FILE* handle = fopen(file, "a");
    fprintf(handle, "here we go\n");
    fclose(handle);
}
