add_executable(libexec_unit_test
        ArrayTest.cc
        SessionTest.cc
        ExecutorTest.cc
        StorageTest.cc
)

target_include_directories(libexec_unit_test INTERFACE ${GTEST_INCLUDE_DIRS})
target_link_libraries(libexec_unit_test exec_a)
target_link_libraries(libexec_unit_test ${GTEST_BOTH_LIBRARIES})

add_test(NAME LibExecUnitTests COMMAND libexec_unit_test)
