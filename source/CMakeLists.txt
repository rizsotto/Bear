include(CheckSymbolExists)
include(CheckIncludeFile)
check_symbol_exists(_NSGetEnviron crt_externs.h HAVE_NSGETENVIRON)
check_include_file(xlocale.h HAVE_XLOCALE_HEADER)
check_include_file(spawn.h HAVE_SPAWN_HEADER)

configure_file(config.h.in ${CMAKE_CURRENT_BINARY_DIR}/config.h)


include_directories(${CMAKE_CURRENT_BINARY_DIR})
include_directories(${CMAKE_CURRENT_SOURCE_DIR})


set(EAR_LIB_FILE ${CMAKE_SHARED_LIBRARY_PREFIX}ear${CMAKE_SHARED_LIBRARY_SUFFIX})
set(EAR_LIB_PATH "${CMAKE_INSTALL_LIBDIR}/bear")
set(DEFAULT_PRELOAD_FILE ${CMAKE_INSTALL_PREFIX}/${EAR_LIB_PATH}/${EAR_LIB_FILE}
        CACHE STRING "Default path to libear.")
set(PEAR_EXE ${CMAKE_INSTALL_LIBEXECDIR}/pear
        CACHE STRING "Default path to pear")


add_subdirectory(libear_a)
add_subdirectory(libear)
add_subdirectory(libpear_a)
add_subdirectory(pear)
add_subdirectory(cpear)
add_subdirectory(bear)
add_subdirectory(gtest_main_a)
