add_executable(libintercept_unit_test
        EnvironmentTest.cc
        ResultTest.cc
        SessionTest.cc
)

target_link_libraries(libintercept_unit_test intercept_a)
target_link_libraries(libintercept_unit_test gtest gtest_main ${CMAKE_THREAD_LIBS_INIT})

add_test(NAME LibInterceptUnitTests COMMAND libintercept_unit_test)
