# Create a static library, which is used for unit tests and the final shared library.
add_library(flags_a STATIC
        Flags.cc
        )

target_include_directories(flags_a PUBLIC ../include/)
target_link_libraries(flags_a PUBLIC result_a)
target_link_libraries(flags_a PRIVATE fmt::fmt)
target_compile_features(flags_a PUBLIC cxx_std_17)
target_compile_options(flags_a PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(flags_a
        PROPERTIES
        PUBLIC_HEADERS ../include/Flags.h)
