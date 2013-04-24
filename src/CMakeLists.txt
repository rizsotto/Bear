# This file is distributed under MIT-LICENSE. See COPYING for details.

include(GNUInstallDirs)

add_definitions(-D_GNU_SOURCE)
add_definitions(-DDEFAULT_PRELOAD_FILE=${CMAKE_INSTALL_PREFIX}/${CMAKE_INSTALL_LIBDIR}/${CMAKE_SHARED_LIBRARY_PREFIX}ear${CMAKE_SHARED_LIBRARY_SUFFIX})
add_definitions(-DDEFAULT_SOCKET_FILE=/tmp/bear.socket)
add_definitions(-DDEFAULT_OUTPUT_FILE=compile_commands.json)


add_library(ear SHARED
    stringarray.c
    protocol.c
    environ.c
    execs.c)
target_link_libraries(ear ${CMAKE_DL_LIBS})
set_target_properties(ear PROPERTIES
    COMPILE_DEFINITIONS "CLIENT")
if (CMAKE_COMPILER_IS_GNUCC)
    set_target_properties(ear PROPERTEIS
        LINK_FLAGS "-Wl,--version-script=${CMAKE_CURRENT_SOURCE_DIR}/ExportedSymbolsList")
endif (CMAKE_COMPILER_IS_GNUCC)


add_executable(bear
    stringarray.c
    protocol.c
    main.c
    output.c
    json.c)
set_target_properties(bear PROPERTIES
    COMPILE_DEFINITIONS "SERVER")

include(CheckFunctionExists)
check_function_exists(execve HAVE_EXECVE)
check_function_exists(execv HAVE_EXECV)
check_function_exists(execvpe HAVE_EXECVPE)
check_function_exists(execvp HAVE_EXECVP)
check_function_exists(execl HAVE_EXECL)
check_function_exists(execlp HAVE_EXECLP)
check_function_exists(execle HAVE_EXECLE)
configure_file(${CMAKE_CURRENT_SOURCE_DIR}/config.h.in ${CMAKE_CURRENT_BINARY_DIR}/config.h)
include_directories(${CMAKE_CURRENT_BINARY_DIR})


install(TARGETS bear
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
install(TARGETS ear
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR})
