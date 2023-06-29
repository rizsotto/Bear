#!/usr/bin/env sh

# REQUIRES: shell

# RUN: printf "int foo() { return 1; }" > %T/libsome_dir_for_libs.c
# RUN: gcc -c %T/libsome_dir_for_libs.c -o %T/libsome_dir_for_libs.o
# RUN: ar -q -c %T/libsome_dir_for_libs.a %T/libsome_dir_for_libs.o

# RUN: mkdir -p %T/other
# RUN: printf "int foo() { return 1; }" > %T/other/libsome_dir_for_libs.c
# RUN: gcc -c %T/other/libsome_dir_for_libs.c -o %T/other/libsome_dir_for_libs.o
# RUN: ar -q -c %T/other/libsome_dir_for_libs.a %T/other/libsome_dir_for_libs.o

# RUN: cd %T; %{bear} --verbose --output %t.json --config %t.config.json -- %{shell} %s %t
# RUN: assert_compilation %t.json count -eq 1
# RUN: assert_compilation %t.json contains -file %T/some_dir_for_libs.c -files %T/other/libsome_dir_for_libs.a -directory %T -arguments %{c_compiler} -c -L ./other -L. -lsome_dir_for_libs -o some_dir_for_libs.c.o some_dir_for_libs.c
# RUN: assert_compilation %t_link.json count -eq 1
# RUN: assert_compilation %t_link.json contains -files %T/other/libsome_dir_for_libs.a %T/some_dir_for_libs.c.o -directory %T -arguments %{c_compiler} -L ./other -L. -lsome_dir_for_libs some_dir_for_libs.c.o -o some_dir_for_libs

cat > "$1.config.json" << EOF
{
  "linking": {
    "filename": "$1_link.json"
  }
}
EOF

echo "int main() { return 0; }" > some_dir_for_libs.c

$CC -o some_dir_for_libs -L ./other -L. -lsome_dir_for_libs some_dir_for_libs.c
