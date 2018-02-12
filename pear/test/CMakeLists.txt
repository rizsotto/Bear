find_package(GTest REQUIRED)

add_executable(pear_test
        ./ResultTest.cc
        ../source/Result.h)
target_link_libraries(pear_test GTest::GTest GTest::Main)

enable_testing()
add_test(NAME PearUnitTests COMMAND pear_test)
