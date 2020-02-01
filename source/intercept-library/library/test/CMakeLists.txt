add_executable(libexec_unit_test
        ArrayTest.cc
        ExecutorTest.cc
        SessionTest.cc
        StorageTest.cc
)

target_include_directories(libexec_unit_test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/../source)
target_include_directories(libexec_unit_test PRIVATE ${CMAKE_CURRENT_BINARY_DIR}/../source)
target_link_libraries(libexec_unit_test exec_a)
target_link_libraries(libexec_unit_test gtest gtest_main ${CMAKE_THREAD_LIBS_INIT})

add_test(NAME libexec_unit_test COMMAND libexec_unit_test)
