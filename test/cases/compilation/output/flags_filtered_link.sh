#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} --verbose --output %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 4
# RUN: assert_compilation %t.json contains -file %T/flags_filtered_link_1.c -directory %T -arguments %{c_compiler} -c -o flags_filtered_link_1.o -fpic flags_filtered_link_1.c
# RUN: assert_compilation %t.json contains -file %T/flags_filtered_link_2.c -directory %T -arguments %{c_compiler} -c -o flags_filtered_link_2.o -fpic flags_filtered_link_2.c
# RUN: assert_compilation %t.json contains -file %T/flags_filtered_link_3.c -directory %T -arguments %{c_compiler} -c -o flags_filtered_link_3 flags_filtered_link_3.c
# RUN: assert_compilation %t.json contains -file %T/flags_filtered_link_4.c -directory %T -arguments %{c_compiler} -c -o flags_filtered_link_4 flags_filtered_link_4.c

# set up platform specific linker options
PREFIX="foobar";
if [ $(uname | grep -i "darwin") ]; then
  LD_FLAGS="-o lib${PREFIX}.dylib -dynamiclib -install_name @rpath/${PREFIX}"
else
  LD_FLAGS="-o lib${PREFIX}.so -shared -Wl,-soname,${PREFIX}"
fi

# create the source files
echo "int foo() { return 2; }" > flags_filtered_link_1.c;
echo "int bar() { return 2; }" > flags_filtered_link_2.c;
echo "int main() { return 0; }" > flags_filtered_link_3.c;
echo "int main() { return 0; }" > flags_filtered_link_4.c;

$CC -c -o flags_filtered_link_1.o -fpic flags_filtered_link_1.c;
$CC -c -o flags_filtered_link_2.o -fpic flags_filtered_link_2.c;
$CC ${LD_FLAGS} flags_filtered_link_1.o flags_filtered_link_2.o;

$CC -o flags_filtered_link_3 -l${PREFIX}  -L.  flags_filtered_link_3.c;
$CC -o flags_filtered_link_4 -l ${PREFIX} -L . flags_filtered_link_4.c;
