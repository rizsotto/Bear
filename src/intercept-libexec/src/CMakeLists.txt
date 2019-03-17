include(CheckSymbolExists)
include(CheckIncludeFile)
check_symbol_exists(_NSGetEnviron crt_externs.h HAVE_NSGETENVIRON)
check_include_file(spawn.h HAVE_SPAWN_HEADER)

configure_file(config.h.in ${CMAKE_CURRENT_BINARY_DIR}/config.h)


include_directories(${CMAKE_CURRENT_BINARY_DIR})
include_directories(${CMAKE_CURRENT_SOURCE_DIR})

add_subdirectory(libexec_a)
add_subdirectory(libexec)
