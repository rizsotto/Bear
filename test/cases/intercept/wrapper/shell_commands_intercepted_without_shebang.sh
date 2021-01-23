#!/usr/bin/env sh

# REQUIRES: shell
# RUN: %{shell} %s > %t.sh
# RUN: chmod +x %t.sh
# RUN: cd %T; %{intercept} --force-wrapper --verbose --output %t.sqlite3 -- %t.sh
# RUN: assert_intercepted %t.sqlite3 count -ge 3
# RUN: assert_intercepted %t.sqlite3 contains -program %{c_compiler} -arguments %{c_compiler} -c shell_commands_intercepted.c -o shell_commands_intercepted.1.o
# RUN: assert_intercepted %t.sqlite3 contains -program %{c_compiler} -arguments %{c_compiler} -c shell_commands_intercepted.c -o shell_commands_intercepted.2.o
# RUN: assert_intercepted %t.sqlite3 contains -program %{c_compiler} -arguments %{c_compiler} -c shell_commands_intercepted.c -o shell_commands_intercepted.3.o

cat <<EOF
touch shell_commands_intercepted.c

\$CC -c shell_commands_intercepted.c -o shell_commands_intercepted.1.o;
\$CC -c shell_commands_intercepted.c -o shell_commands_intercepted.2.o;
\$CC -c shell_commands_intercepted.c -o shell_commands_intercepted.3.o;
EOF
