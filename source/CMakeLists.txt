set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_IMPLICIT_LINK_LIBRARIES "")
set(CMAKE_CXX_IMPLICIT_LINK_DIRECTORIES "")
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-exceptions -fno-rtti")

include(CheckFunctionExists)
include(CheckSymbolExists)
include(CheckIncludeFile)
check_function_exists(execve HAVE_EXECVE)
check_function_exists(execv HAVE_EXECV)
check_function_exists(execvpe HAVE_EXECVPE)
check_function_exists(execvp HAVE_EXECVP)
check_function_exists(execvP HAVE_EXECVP2)
check_function_exists(exect HAVE_EXECT)
check_function_exists(execl HAVE_EXECL)
check_function_exists(execlp HAVE_EXECLP)
check_function_exists(execle HAVE_EXECLE)
check_function_exists(posix_spawn HAVE_POSIX_SPAWN)
check_function_exists(posix_spawnp HAVE_POSIX_SPAWNP)
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
add_subdirectory(bear)
add_subdirectory(gtest_main_a)
