#!/usr/bin/env sh

# UNSUPPORTED: is_darwin

# RUN: touch %T/libflag_wl_1.%{dynamic_lib_extension}
# RUN: ar -q -c %T/libflag_wl_2.a
# RUN: touch %T/libflag_wl_2.%{dynamic_lib_extension}

# RUN: mkdir -p %T/other
# RUN: ar -q -c %T/other/libflag_wl_3.a
# RUN: ar -q -c %T/other/libflag_wl_1.a
# RUN: touch %T/other/libflag_wl_3.%{dynamic_lib_extension}
# RUN: touch %T/other/libflag_wl_1.%{dynamic_lib_extension}

# RUN: cd %T; %{bear} --verbose --with-link --output-compile %t.json --output-link %t_link.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 1
# RUN: assert_compilation %t.json contains -file %T/flag_wl.c -files %T/other/libflag_wl_3.a %T/other/libflag_wl_1.a %T/other/libflag_wl_1.%{dynamic_lib_extension} %T/libflag_wl_2.%{dynamic_lib_extension} -directory %T -arguments %{c_compiler} -c -L ./other/ -Wl,-Bdynamic,-Bstatic -lflag_wl_3 -lflag_wl_1 -L. -Wl,-Bdynamic -lflag_wl_1 -lflag_wl_2 -o flag_wl.c.o flag_wl.c
# RUN: assert_compilation %t_link.json count -eq 1
# RUN: assert_compilation %t_link.json contains -files %T/flag_wl.c.o %T/other/libflag_wl_3.a %T/other/libflag_wl_1.a %T/other/libflag_wl_1.%{dynamic_lib_extension} %T/libflag_wl_2.%{dynamic_lib_extension} -directory %T -arguments %{c_compiler} flag_wl.c.o -L ./other/ -Wl,-Bdynamic,-Bstatic -lflag_wl_3 -lflag_wl_1 -L. -Wl,-Bdynamic -lflag_wl_1 -lflag_wl_2 -o flag_wl

echo "int main() { return 0; }" > flag_wl.c

$CC -o flag_wl flag_wl.c -L ./other/ -Wl,-Bdynamic,-Bstatic -lflag_wl_3 -lflag_wl_1 -L. -Wl,-Bdynamic -lflag_wl_1 -lflag_wl_2
