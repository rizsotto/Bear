# This file is distributed under MIT-LICENSE. See COPYING for details.

include(GNUInstallDirs)

add_definitions(-D_GNU_SOURCE)
add_definitions(-DDEFAULT_PRELOAD_FILE=${CMAKE_INSTALL_PREFIX}/${CMAKE_INSTALL_LIBDIR}/libear.so)
add_definitions(-DDEFAULT_SOCKET_FILE=/tmp/bear.socket)
add_definitions(-DDEFAULT_OUTPUT_FILE=compile_commands.json)


add_library(ear SHARED
    stringarray.c
    protocol.c
    envarray.c
    execs.c
    report.c)
target_link_libraries(ear dl)
set_target_properties(ear PROPERTIES
    LINK_FLAGS "-Wl,--version-script=${CMAKE_CURRENT_SOURCE_DIR}/ExportedSymbolsList"
    COMPILE_DEFINITIONS "CLIENT")


add_executable(bear
    stringarray.c
    protocol.c
    main.c
    cdb.c
    json.c)
set_target_properties(bear PROPERTIES
    COMPILE_DEFINITIONS "SERVER")


install(TARGETS bear
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
install(TARGETS ear
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR})
