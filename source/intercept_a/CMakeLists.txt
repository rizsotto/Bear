set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_IMPLICIT_LINK_LIBRARIES "")
set(CMAKE_CXX_IMPLICIT_LINK_DIRECTORIES "")
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-exceptions -fno-rtti")

add_library(intercept_a STATIC
        Environment.h
        Environment.cc
        Reporter.h
        Reporter.cc
        Result.h
        Parameters.h
        SystemCalls.h
        SystemCalls.cc
)

add_subdirectory(test)
