# Copyright 2012 by Laszlo Nagy [see file MIT-LICENSE]

cmake_minimum_required(VERSION 2.6)
project(buildspy)


set(ourlib_target buildspy)
set(ourlib_symbols ${CMAKE_CURRENT_SOURCE_DIR}/ExportedSymbolsList)

add_library(${ourlib_target} SHARED buildspy.c)

set_target_properties(${ourlib_target} PROPERTIES
    LINKER_LANGUAGE C)
set_target_properties(${ourlib_target} PROPERTIES
    COMPILE_FLAGS -nostdlib)
set_target_properties(${ourlib_target} PROPERTIES
    LINK_FLAGS "-Wl,--version-script=${ourlib_symbols}")
target_link_libraries(${ourlib_target} dl)

install(TARGETS ${ourlib_target}
    LIBRARY DESTINATION lib)
