add_library(result_a INTERFACE)
target_include_directories(result_a INTERFACE include/)

if (ENABLE_UNIT_TESTS)
    add_executable(result_unit_test
            test/ResultTest.cc
            )

    target_link_libraries(result_unit_test result_a)
    target_link_libraries(result_unit_test PkgConfig::GTest ${CMAKE_THREAD_LIBS_INIT})

    add_test(NAME bear::result_unit_test COMMAND $<TARGET_FILE:result_unit_test>)
endif ()