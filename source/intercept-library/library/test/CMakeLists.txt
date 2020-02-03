add_executable(libexec_unit_test
        ArrayTest.cc
        ExecutorTest.cc
        SessionTest.cc
        StorageTest.cc
        )

target_include_directories(libexec_unit_test PRIVATE ${LIB_CFG_DIR})
target_include_directories(libexec_unit_test PRIVATE ${LIB_SRC_DIR})
target_include_directories(libexec_unit_test PRIVATE ${EXE_API_DIR})
target_link_libraries(libexec_unit_test exec_a)
target_link_libraries(libexec_unit_test gtest gtest_main ${CMAKE_THREAD_LIBS_INIT})
target_link_libraries(libexec_unit_test gmock)

add_test(NAME libexec_unit_test COMMAND libexec_unit_test)
