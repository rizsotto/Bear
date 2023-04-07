#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} --verbose --output-compile %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -ge 1
# RUN: assert_compilation %t.json contains -file %T/define_with_quote.c -directory %T -arguments %{cxx_compiler} -c -DEXPORT="extern \"C\"" -o define_with_quote.c.o define_with_quote.c

cat > define_with_quote.c <<EOF
#include <cstdio>

EXPORT void foo(void) {
  printf("Hello world!\n");
}

int main() {
  foo();
}
EOF

$CXX -DEXPORT="extern \"C\"" -o define_with_quote define_with_quote.c;
