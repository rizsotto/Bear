# This file is distributed under MIT-LICENSE. See COPYING for details.

add_subdirectory(unit_level)
add_subdirectory(end-to-end)

add_custom_target(check)
add_dependencies(check unit_level)
add_dependencies(check end-to-end)
