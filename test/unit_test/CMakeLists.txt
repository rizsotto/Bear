link_directories(${LIBCONFIG_LIBRARY_DIRS})
add_executable(unit_test EXCLUDE_FROM_ALL
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
include_directories(${LIBCONFIG_INCLUDE_DIRS})
target_link_libraries(unit_test ${LIBCONFIG_LIBRARIES})

add_test(unit_test ./unit_test)
add_dependencies(check unit_test)
