#!/usr/bin/env sh

# REQUIRES: preload, shell
# RUN: %{shell} %s %t
# RUN: cd %T; /usr/bin/env - %{bear} --verbose --output-compile %t.json -- %{shell} %t/build.sh
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file %t/source_1.c -directory %T -arguments %{c_compiler} -c -o %t/source_1.o %t/source_1.c
# RUN: assert_compilation %t.json contains -file %t/source_2.c -directory %T -arguments %{c_compiler} -c -o %t/source_2.o %t/source_2.c

TEST=$1

mkdir -p $TEST;
touch $TEST/source_1.c;
touch $TEST/source_2.c;

cat > "$TEST/build.sh" << EOF
#!/usr/bin/env sh

$CC -c -o $TEST/source_1.o $TEST/source_1.c;
$CC -c -o $TEST/source_2.o $TEST/source_2.c;
EOF
