# This file is distributed under MIT-LICENSE. See COPYING for details.

add_executable(unit_test
    main.c
    ../../src/stringarray.c
    ../../src/protocol.c
    ../../src/json.c
    ../../src/envarray.c
    )
add_definitions(-D_GNU_SOURCE)
include_directories(../../src)

add_test(unit_test ./unit_test)
add_dependencies(check unit_test)
