# This file is distributed under MIT-LICENSE. See COPYING for details.

set(ourlib_target common)

add_library(${ourlib_target} STATIC
    stringarray.c
    protocol.c)
add_definitions(-D_GNU_SOURCE)

set_target_properties(${ourlib_target} PROPERTIES
    LINKER_LANGUAGE C)
set_target_properties(${ourlib_target} PROPERTIES
    COMPILE_FLAGS -nostdlib)
set_target_properties(${ourlib_target} PROPERTIES
    COMPILE_FLAGS -fPIC)
