#!/usr/bin/env sh

# REQUIRES: shell, dynamic-shell
# RUN: cd %T; %{bear} -vvvv --cdb %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 1
# RUN: assert_compilation %t.json contains -file define_with_escaped_quote.c -directory %T -arguments %{c_compiler} -c '-DMESSAGE="Hello World\n"' -o define_with_escaped_quote define_with_escaped_quote.c

cat > define_with_escaped_quote.c <<EOF
#include <stdio.h>

int main() {
  printf(MESSAGE);
  return 0;
}
EOF

$CC '-DMESSAGE="Hello World\n"' -o define_with_escaped_quote define_with_escaped_quote.c;
