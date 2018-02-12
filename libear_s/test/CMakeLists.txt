find_package(GTest REQUIRED)

add_executable(ear_test
        ./ArrayTest.cc
        ./EnvironmentTest.cc
        ./ExecutorTest.cc
        ./StringTest.cc
        ../source/Array.h
        ../source/String.h
        ../source/Environment.h
        ../source/DynamicLinker.h
        ../source/Executor.h)
target_link_libraries(ear_test GTest::GTest GTest::Main)

enable_testing()
add_test(NAME EarUnitTests COMMAND ear_test)
