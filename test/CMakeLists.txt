# This file is distributed under MIT-LICENSE. See COPYING for details.

add_subdirectory(unit_test)
add_subdirectory(end-to-end)
add_subdirectory(exec_anatomy)

add_custom_target(check)
add_dependencies(check run_unit_test)
add_dependencies(check run_exec_anatomy)
add_dependencies(check end-to-end)
