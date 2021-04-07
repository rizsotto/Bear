#!/usr/bin/env sh

# REQUIRES: shell
# RUN: echo "#!/usr/bin/env sh\n%{c_compiler} $@" > %t.wrapper
# RUN: chmod +x %t.wrapper
# RUN: cd %T; env CC=%t.wrapper %{intercept} --force-wrapper --verbose --output %t.events.db -- %{shell} %s || true
# RUN: %{events_db} dump --path %t.events.db --output %t.json
# RUN: assert_intercepted %t.json count -ge 1
# RUN: assert_intercepted %t.json contains -program %t.wrapper -arguments %t.wrapper -c use_env.c -o use_env.o

touch use_env.c

$CC -c use_env.c -o use_env.o;
