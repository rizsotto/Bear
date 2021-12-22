#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell, libtool
# RUN: %{intercept} --force-preload --verbose --output %t.json -- %{shell} %s
# RUN: assert_intercepted %t.json count -ge 4
# RUN: assert_intercepted %t.json contains -program %{c_compiler} -arguments %{c_compiler} -g -O -c main.c -o main.o
# RUN: assert_intercepted %t.json contains -program %{c_compiler} -arguments %{c_compiler} -g -O -c hello.c -o hello.o

echo "int main() { return 0; }" > main.c
echo "int hello() { return 1; }" > hello.c

$LIBTOOL --mode=compile --tag=CC $CC -g -O -c main.c -o main.o;
$LIBTOOL --mode=compile --tag=CC $CC -g -O -c hello.c -o hello.o;
$LIBTOOL --mode=link --tag=CC $CC -g -O -o libhello.la hello.lo
$LIBTOOL --mode=link --tag=CC $CC -g -O -o libtool_test main.lo libhello.la
