add_library(intercept_a STATIC
        Environment.h
        Environment.cc
        Interface.h
        Reporter.h
        Reporter.cc
        Result.h
        Session.h
        Session.cc
        SystemCalls.h
        SystemCalls.cc
)

target_compile_features(intercept_a PUBLIC cxx_std_17)
target_compile_options(intercept_a PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(intercept_a PROPERTIES LINKER_LANGUAGE "C")

add_subdirectory(test)
