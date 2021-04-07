// REQUIRES: preload, shell, dynamic-shell
// RUN: mkdir -p %T/same_name_in_path/a %T/same_name_in_path/b
// RUN: %{compile} '-D_MESSAGE="one"' -o %T/same_name_in_path/a/a.out %s
// RUN: %{compile} '-D_MESSAGE="two"' -o %T/same_name_in_path/b/a.out %s
// RUN: env PATH=%T/same_name_in_path/a:%T/same_name_in_path/b %{shell} -c a.out > %t.without.txt
// RUN: env PATH=%T/same_name_in_path/a:%T/same_name_in_path/b %{intercept} --output %t.events.db -- %{shell} -c a.out > %t.with.txt
// RUN: diff %t.without.txt %t.with.txt

#include <stdio.h>

int main()
{
    const char *const message = _MESSAGE;
    printf(message);

    return 0;
}
