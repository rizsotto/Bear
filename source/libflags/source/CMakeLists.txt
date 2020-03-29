# Create a static library, which is used for unit tests and the final shared library.
add_library(flags_a STATIC
        ../include/Flags.h
        Flags.cc
        )

target_include_directories(flags_a PUBLIC ${LIBFLAGS_INCLUDE_DIR})
target_include_directories(flags_a PRIVATE ${LIBRESULT_INCLUDE_DIR})
target_link_libraries(flags_a PRIVATE fmt::fmt)
target_compile_features(flags_a PUBLIC cxx_std_17)
target_compile_options(flags_a PUBLIC -fno-exceptions -fno-rtti)
