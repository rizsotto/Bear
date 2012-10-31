# This file is distributed under MIT-LICENSE. See COPYING for details.

add_executable(bear main.c)
add_definitions(-D_GNU_SOURCE)

set_target_properties(bear PROPERTIES
    LINKER_LANGUAGE C)

install(TARGETS bear
    RUNTIME DESTINATION bin)
