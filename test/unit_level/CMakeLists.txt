# This file is distributed under MIT-LICENSE. See COPYING for details.

add_executable(unit_test main.c)
add_definitions(-D_GNU_SOURCE)
target_link_libraries(unit_test common)
include_directories(../../src/common)

add_custom_target(unit_level
  COMMAND ./unit_test
  COMMENT "Running unit tests")

add_dependencies(unit_level unit_test)
add_dependencies(unit_level ear)
add_dependencies(unit_level bear)
