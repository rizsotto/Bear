# This file is distributed under MIT-LICENSE. See COPYING for details.

include(GNUInstallDirs)

set(CommonSources stringarray.c protocol.c)

add_definitions(-D_GNU_SOURCE)
add_definitions(-DLIBEAR_INSTALL_DIR=${CMAKE_INSTALL_PREFIX}/${CMAKE_INSTALL_LIBDIR})
add_definitions(-DDEFAULT_SOCKET_FILE=/tmp/bear.socket)
add_definitions(-DDEFAULT_OUTPUT_FILE=compile_commands.json)


add_library(ear SHARED
    ${CommonSources}
    envarray.c
    execs.c
    report.c)

set_target_properties(ear PROPERTIES
    LINK_FLAGS "-Wl,--version-script=${CMAKE_CURRENT_SOURCE_DIR}/ExportedSymbolsList")

target_link_libraries(ear dl)


add_executable(bear
    ${CommonSources}
    main.c
    cdb.c
    json.c)


install(TARGETS bear
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
install(TARGETS ear
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR})
