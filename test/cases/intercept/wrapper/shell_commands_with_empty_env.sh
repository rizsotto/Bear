#!/usr/bin/env sh

# XFAIL: *
# REQUIRES: shell
# RUN: cd %T; %{intercept} --force-wrapper --verbose --output %t.sqlite3 -- env - %{shell} %s
# RUN: assert_intercepted %t.sqlite3 count -ge 3
# RUN: assert_intercepted %t.sqlite3 contains -program %{c_compiler} -arguments %{c_compiler} -c shell_commands_with_empty_env.c -o shell_commands_with_empty_env.1.o
# RUN: assert_intercepted %t.sqlite3 contains -program %{c_compiler} -arguments %{c_compiler} -c shell_commands_with_empty_env.c -o shell_commands_with_empty_env.2.o
# RUN: assert_intercepted %t.sqlite3 contains -program %{c_compiler} -arguments %{c_compiler} -c shell_commands_with_empty_env.c -o shell_commands_with_empty_env.3.o

touch shell_commands_with_empty_env.c

CC=${CC:-cc}

$CC -c shell_commands_with_empty_env.c -o shell_commands_with_empty_env.1.o
$CC -c shell_commands_with_empty_env.c -o shell_commands_with_empty_env.2.o
$CC -c shell_commands_with_empty_env.c -o shell_commands_with_empty_env.3.o
