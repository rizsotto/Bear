// REQUIRES: preload, shell, dynamic-shell
// RUN: %{compile} '-D_FILE="%t.state"' -fpic -shared -o %t.so %s
// RUN: env LD_PRELOAD=%t.so %{intercept} --verbose --output %t.sqlite3 -- %{shell} -c %{true}
// RUN: assert_intercepted %t.sqlite3 count -ge 2
// RUN: assert_intercepted %t.sqlite3 contains -arguments %{shell} -c %{true}
// RUN: assert_intercepted %t.sqlite3 contains -program %{true} -arguments %{true}
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
