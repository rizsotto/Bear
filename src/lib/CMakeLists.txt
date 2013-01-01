# This file is distributed under MIT-LICENSE. See COPYING for details.

set(ourlib_target ear)
set(ourlib_symbols ${CMAKE_CURRENT_SOURCE_DIR}/ExportedSymbolsList)

add_library(${ourlib_target} SHARED execs.c report.c)
add_definitions(-D_GNU_SOURCE)

set_target_properties(${ourlib_target} PROPERTIES
    LINKER_LANGUAGE C)
set_target_properties(${ourlib_target} PROPERTIES
    LINK_FLAGS "-Wl,--version-script=${ourlib_symbols}")

target_link_libraries(${ourlib_target} dl)
target_link_libraries(${ourlib_target} common)

install(TARGETS ${ourlib_target}
    LIBRARY DESTINATION lib)
