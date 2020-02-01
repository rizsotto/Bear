# Create a header file with the relevant configuration values.
configure_file(config.h.in ${CMAKE_CURRENT_BINARY_DIR}/config.h)

# Create a static library, which is used for unit tests and the final shared library.
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

target_include_directories(intercept_a PRIVATE ${EXE_CFG_DIR})
target_include_directories(intercept_a PRIVATE ${EXE_API_DIR})
target_include_directories(intercept_a PRIVATE ${LIB_API_DIR})
target_compile_features(intercept_a PUBLIC cxx_std_17)
target_compile_options(intercept_a PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(intercept_a PROPERTIES LINKER_LANGUAGE "C")
