# This file is distributed under MIT-LICENSE. See COPYING for details.

add_executable(unit_test main.c)
add_definitions(-D_GNU_SOURCE)
target_link_libraries(unit_test common)
target_link_libraries(unit_test rt)
include_directories(../../src/common)

add_custom_target(run_unit_test
  COMMAND ./unit_test
  COMMENT "Running unit tests")

add_dependencies(run_unit_test unit_test)
add_dependencies(run_unit_test ear)
add_dependencies(run_unit_test bear)
