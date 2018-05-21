add_library(pear_a STATIC
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
