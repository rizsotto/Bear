# This file is distributed under MIT-LICENSE. See COPYING for details.

add_executable(unit_test main.c)
add_definitions(-D_GNU_SOURCE)

set_target_properties(unit_test PROPERTIES
    LINKER_LANGUAGE C)

add_custom_target(unit_level
  COMMAND ./unit_test
  COMMENT "Running unit tests")

add_dependencies(unit_level unit_test)
add_dependencies(unit_level ear)
add_dependencies(unit_level bear)
