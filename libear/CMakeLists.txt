include(CheckFunctionExists)
include(CheckSymbolExists)
check_function_exists(vfork HAVE_VFORK)
check_function_exists(execve HAVE_EXECVE)
check_function_exists(execv HAVE_EXECV)
check_function_exists(execvpe HAVE_EXECVPE)
check_function_exists(execvp HAVE_EXECVP)
check_function_exists(execvP HAVE_EXECVP2)
check_function_exists(execl HAVE_EXECL)
check_function_exists(execlp HAVE_EXECLP)
check_function_exists(execle HAVE_EXECLE)
check_function_exists(posix_spawn HAVE_POSIX_SPAWN)
check_function_exists(posix_spawnp HAVE_POSIX_SPAWNP)
check_symbol_exists(_NSGetEnviron crt_externs.h HAVE_NSGETENVIRON)

configure_file(${CMAKE_CURRENT_SOURCE_DIR}/config.h.in ${CMAKE_CURRENT_BINARY_DIR}/config.h)
include_directories(${CMAKE_CURRENT_BINARY_DIR})

add_library(eark SHARED ear.c)
target_link_libraries(eark ${CMAKE_DL_LIBS})

include(GNUInstallDirs)
install(TARGETS eark
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR})
