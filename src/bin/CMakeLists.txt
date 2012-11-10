# This file is distributed under MIT-LICENSE. See COPYING for details.

add_executable(bear main.c write.c)
add_definitions(-D_GNU_SOURCE)
add_definitions(-DLIBEAR_INSTALL_DIR=${CMAKE_INSTALL_PREFIX}/lib)
add_definitions(-DDEFAULT_SOCKET_FILE=/tmp/bear.socket)
add_definitions(-DDEFAULT_OUTPUT_FILE=compile_commands.json)

set_target_properties(bear PROPERTIES
    LINKER_LANGUAGE C)

install(TARGETS bear
    RUNTIME DESTINATION bin)
