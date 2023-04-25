#!/usr/bin/env sh

# REQUIRES: shell

# RUN: cd %T; %{bear} --verbose --with-link --output-compile %t.json --output-link %t_link.json -- %{shell} %s -build
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file %T/append/src/source_1.c -directory %T -arguments %{c_compiler} -c -o append/src/source_1.o append/src/source_1.c
# RUN: assert_compilation %t.json contains -file %T/append/src/source_2.c -directory %T -arguments %{c_compiler} -c -o append/src/source_2.o append/src/source_2.c
# RUN: assert_compilation %t_link.json count -eq 1
# RUN: assert_compilation %t_link.json contains -files %T/append/src/source_1.o %T/append/src/source_2.o -directory %T -arguments %{c_compiler} append/src/source_1.o append/src/source_2.o -o src

# RUN: cd %T; %{bear} --verbose --with-link --output-compile %t.json --output-link %t_link.json --append -- %{shell} %s -test
# RUN: assert_compilation %t.json count -eq 4
# RUN: assert_compilation %t.json contains -file %T/append/src/source_1.c -directory %T -arguments %{c_compiler} -c -o append/src/source_1.o append/src/source_1.c
# RUN: assert_compilation %t.json contains -file %T/append/src/source_2.c -directory %T -arguments %{c_compiler} -c -o append/src/source_2.o append/src/source_2.c
# RUN: assert_compilation %t.json contains -file %T/append/test/source_1.c -directory %T -arguments %{c_compiler} -c -o append/test/source_1.o append/test/source_1.c
# RUN: assert_compilation %t.json contains -file %T/append/test/source_2.c -directory %T -arguments %{c_compiler} -c -o append/test/source_2.o append/test/source_2.c
# RUN: assert_compilation %t_link.json count -eq 2
# RUN: assert_compilation %t_link.json contains -files %T/append/src/source_1.o %T/append/src/source_2.o -directory %T -arguments %{c_compiler} append/src/source_1.o append/src/source_2.o -o src
# RUN: assert_compilation %t_link.json contains -files %T/append/test/source_1.o %T/append/test/source_2.o -directory %T -arguments %{c_compiler} append/test/source_1.o append/test/source_2.o -o test

# RUN: cd %T; %{bear} --verbose --output-compile %t.json --append -- %{shell} %s -clean
# RUN: assert_compilation %t.json count -eq 0
# RUN: assert_compilation %t_link.json count -eq 2

build()
{
  mkdir -p append append/src
  touch append/src/source_1.c append/src/source_2.c
  $CC -c -o append/src/source_1.o append/src/source_1.c
  $CC -c -o append/src/source_2.o append/src/source_2.c
  $CC -o src append/src/source_1.o append/src/source_2.o
}

verify()
{
  mkdir -p append append/test
  touch append/test/source_1.c append/test/source_2.c
  $CC -c -o append/test/source_1.o append/test/source_1.c
  $CC -c -o append/test/source_2.o append/test/source_2.c
  $CC -o test append/test/source_1.o append/test/source_2.o
}

clean()
{
  rm -rf append
}

case $1 in
  -build)
    build
    ;;
  -test)
    verify
    ;;
  -clean)
    clean
    ;;
  *)
    # unknown option
    ;;
esac

true
