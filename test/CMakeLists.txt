cmake_minimum_required(VERSION 3.5 FATAL_ERROR)
if (${CMAKE_VERSION} VERSION_LESS 3.12)
    cmake_policy(VERSION ${CMAKE_MAJOR_VERSION}.${CMAKE_MINOR_VERSION})
else ()
    cmake_policy(VERSION 3.12)
endif ()

project(BearTest LANGUAGES C)

set(CMAKE_REQUIRED_LIBRARIES ${CMAKE_DL_LIBS})
set(CMAKE_REQUIRED_FLAGS -D_GNU_SOURCE)
include(CheckIncludeFile)
check_include_file(spawn.h HAVE_SPAWN_H)
check_include_file(unistd.h HAVE_UNISTD_H)
check_include_file(sys/wait.h HAVE_SYS_WAIT_H)
check_include_file(sys/types.h HAVE_SYS_TYPES_H)
include(CheckSymbolExists)
check_symbol_exists(execve "unistd.h" HAVE_EXECVE)
check_symbol_exists(execv "unistd.h" HAVE_EXECV)
check_symbol_exists(execvpe "unistd.h" HAVE_EXECVPE)
check_symbol_exists(execvp "unistd.h" HAVE_EXECVP)
check_symbol_exists(execvP "unistd.h" HAVE_EXECVP2)
check_symbol_exists(exect "unistd.h" HAVE_EXECT)
check_symbol_exists(execl "unistd.h" HAVE_EXECL)
check_symbol_exists(execlp "unistd.h" HAVE_EXECLP)
check_symbol_exists(execle "unistd.h" HAVE_EXECLE)
check_symbol_exists(execveat "unistd.h" HAVE_EXECVEAT)
check_symbol_exists(fexecve "unistd.h" HAVE_FEXECVE)
check_symbol_exists(posix_spawn "spawn.h" HAVE_POSIX_SPAWN)
check_symbol_exists(posix_spawnp "spawn.h" HAVE_POSIX_SPAWNP)
check_symbol_exists(popen "stdio.h" HAVE_POPEN)
check_symbol_exists(system "stdlib.h" HAVE_SYSTEM)

configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/config.h.in
    ${CMAKE_CURRENT_BINARY_DIR}/config.h)

configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/lit.site.cfg.in
    ${CMAKE_CURRENT_BINARY_DIR}/lit.site.cfg
    @ONLY)

enable_testing()

message(STATUS "Looking for lit")
find_program(LIT_EXECUTABLE
        NAMES lit
        PATHS ENV LIT_PATH)

if (LIT_EXECUTABLE)
    message(STATUS "Looking for lit -- ${LIT_EXECUTABLE}")

    include(GNUInstallDirs)
    add_test(NAME bear::func_test
            COMMAND ${LIT_EXECUTABLE}
            -D_TEST_EXEC_ROOT=${CMAKE_CURRENT_BINARY_DIR}
            -D_BIN_BEAR=${STAGED_INSTALL_PREFIX}/${CMAKE_INSTALL_BINDIR}/bear
            -D_BIN_INTERCEPT=${STAGED_INSTALL_PREFIX}/${CMAKE_INSTALL_BINDIR}/intercept
            -D_BIN_ER=${STAGED_INSTALL_PREFIX}/${CMAKE_INSTALL_LIBEXECDIR}/er
            -D_BIN_WRAPPER=${STAGED_INSTALL_PREFIX}/${CMAKE_INSTALL_LIBEXECDIR}/wrapper.d
            -D_LIB_EXEC=${STAGED_INSTALL_PREFIX}/${CMAKE_INSTALL_LIBDIR}/${CMAKE_SHARED_LIBRARY_PREFIX}exec${CMAKE_SHARED_LIBRARY_SUFFIX}
            -v
            ${CMAKE_CURRENT_BINARY_DIR})

    set_tests_properties(bear::func_test PROPERTIES
            TIMEOUT 60
            )
endif ()
