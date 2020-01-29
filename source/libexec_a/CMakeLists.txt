add_library(exec_a STATIC
        Array.h
        Environment.h
        Environment.cc
        Executor.h
        Executor.cc
        Resolver.h
        Session.h
        Session.cc
        Storage.h
        Storage.cc
)

target_compile_features(exec_a PUBLIC cxx_std_14)
target_compile_options(exec_a PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(exec_a PROPERTIES
        LINKER_LANGUAGE "C"
        POSITION_INDEPENDENT_CODE 1)

add_subdirectory(test)
