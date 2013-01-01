# This file is distributed under MIT-LICENSE. See COPYING for details.

add_executable(exec_anatomy main.c)
add_definitions(-D_GNU_SOURCE)

set(BEAR_EXE ../../src/bin/bear)
set(BEAR_LIB ../../src/lib/libear.so)

add_custom_target(run_exec_anatomy
  COMMAND touch execve.c execv.c execvpe.c execvp.c execl.c execlp.c execle.c
  COMMAND ${BEAR_EXE} -b ${BEAR_LIB} -s ./socket -- ./exec_anatomy
  COMMENT "Running exec anatomy")

add_dependencies(run_exec_anatomy exec_anatomy)
add_dependencies(run_exec_anatomy ear)
add_dependencies(run_exec_anatomy bear)
