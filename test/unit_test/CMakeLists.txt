add_executable(unit_test
    main.c
    ../../src/stringarray.c
    ../../src/protocol.c
    ../../src/json.c
    ../../src/environ.c
    )
add_definitions(-D_GNU_SOURCE)
include_directories(../../src ${CMAKE_CURRENT_BINARY_DIR}/../../src)
set_target_properties(unit_test PROPERTIES
    COMPILE_DEFINITIONS "CLIENT;SERVER")

add_test(unit_test ./unit_test)
add_dependencies(check unit_test)
