#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} --verbose --output-compile %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file %T/wrapper_1.c -directory %T -arguments %{c_compiler} -c -o wrapper_1.o wrapper_1.c
# RUN: assert_compilation %t.json contains -file %T/wrapper_2.c -directory %T -arguments %{c_compiler} -c -o wrapper_2.o wrapper_2.c

# RUN: cd %T; %{bear} --verbose --output-compile %t.json --force-wrapper -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file %T/wrapper_1.c -directory %T -arguments %{c_compiler} -c -o wrapper_1.o wrapper_1.c
# RUN: assert_compilation %t.json contains -file %T/wrapper_2.c -directory %T -arguments %{c_compiler} -c -o wrapper_2.o wrapper_2.c

touch wrapper_1.c wrapper_2.c

cat > wrapper << EOF
#!/usr/bin/env sh

exec \$*
EOF

chmod +x wrapper

ORIGINAL=$CC
CC=./wrapper

$CC $ORIGINAL -c -o wrapper_1.o wrapper_1.c;
$CC $ORIGINAL -c -o wrapper_2.o wrapper_2.c;
