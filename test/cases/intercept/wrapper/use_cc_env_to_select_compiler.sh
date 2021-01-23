#!/usr/bin/env sh

# REQUIRES: shell
# RUN: echo "#!/usr/bin/env sh\n%{c_compiler} $@" > %t.wrapper
# RUN: chmod +x %t.wrapper
# RUN: cd %T; env CC=%t.wrapper %{intercept} --force-wrapper --verbose --output %t.sqlite3 -- %{shell} %s || true
# RUN: assert_intercepted %t.sqlite3 count -ge 1
# RUN: assert_intercepted %t.sqlite3 contains -program %t.wrapper -arguments %t.wrapper -c use_env.c -o use_env.o

touch use_env.c

$CC -c use_env.c -o use_env.o;
