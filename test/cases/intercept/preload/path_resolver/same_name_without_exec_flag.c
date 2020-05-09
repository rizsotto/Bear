// REQUIRES: preload, shell, dynamic-shell
// RUN: mkdir -p %T/same_name_without_exec_flag/a %T/same_name_without_exec_flag/b
// RUN: touch %T/same_name_without_exec_flag/a/a.out
// RUN: %{compile} '-D_MESSAGE="two"' -o %T/same_name_without_exec_flag/b/a.out %s
// RUN: env PATH=%T/same_name_without_exec_flag/a:%T/same_name_without_exec_flag/b %{shell} -c a.out > %t.without.txt
// RUN: env PATH=%T/same_name_without_exec_flag/a:%T/same_name_without_exec_flag/b %{intercept} --output %t.json -- %{shell} -c a.out > %t.with.txt
// RUN: diff %t.without.txt %t.with.txt

#include <stdio.h>

int main()
{
    const char *const message = _MESSAGE;
    printf(message);

    return 0;
}
