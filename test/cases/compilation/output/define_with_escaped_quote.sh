#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} --verbose --output %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -ge 1
# RUN: assert_compilation %t.json contains -file %T/define_with_escaped_quote.c -directory %T -arguments %{c_compiler} -c '-DMESSAGE="Hello World\n"' -o define_with_escaped_quote.c.o define_with_escaped_quote.c

cat > define_with_escaped_quote.c <<EOF
#include <stdio.h>

int main() {
  printf(MESSAGE);
  return 0;
}
EOF

$CC '-DMESSAGE="Hello World\n"' define_with_escaped_quote.c -o define_with_escaped_quote;
