include(CheckSymbolExists)
include(CheckIncludeFile)
check_symbol_exists(_NSGetEnviron crt_externs.h HAVE_NSGETENVIRON)
check_include_file(spawn.h HAVE_SPAWN_HEADER)

configure_file(config.h.in ${CMAKE_CURRENT_BINARY_DIR}/config.h)


include_directories(${CMAKE_CURRENT_BINARY_DIR})
include_directories(${CMAKE_CURRENT_SOURCE_DIR})

include(GNUInstallDirs)
set(EXEC_LIB_FILE ${CMAKE_SHARED_LIBRARY_PREFIX}exec${CMAKE_SHARED_LIBRARY_SUFFIX})
set(EXEC_LIB_PATH "${CMAKE_INSTALL_LIBDIR}/bear")
set(DEFAULT_PRELOAD_FILE ${CMAKE_INSTALL_PREFIX}/${EXEC_LIB_PATH}/${EXEC_LIB_FILE}
        CACHE STRING "Default path to libexec.")
set(INTERCEPT_EXE ${CMAKE_INSTALL_PREFIX}/${CMAKE_INSTALL_LIBEXECDIR}/intercept
        CACHE STRING "Default path to intercept")


add_subdirectory(libexec_a)
add_subdirectory(libexec)
add_subdirectory(intercept_a)
add_subdirectory(intercept)
add_subdirectory(bear)
