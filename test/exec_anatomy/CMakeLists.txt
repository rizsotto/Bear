add_executable(exec_anatomy main.c)
add_definitions(-D_GNU_SOURCE)
include_directories(${CMAKE_CURRENT_BINARY_DIR}/../../src)

configure_file(
  ${CMAKE_CURRENT_SOURCE_DIR}/run.sh.in
  ${CMAKE_CURRENT_BINARY_DIR}/run.sh
  @ONLY)

add_test(NAME run_exec_anatomy
  COMMAND ./run.sh)

add_dependencies(check exec_anatomy)
add_dependencies(exec_anatomy ear)
add_dependencies(exec_anatomy bear)
