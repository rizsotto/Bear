add_library(result_a INTERFACE
        )

target_include_directories(result_a INTERFACE include/)
target_compile_features(result_a INTERFACE cxx_std_17)
target_compile_options(result_a INTERFACE -fno-exceptions -fno-rtti)

add_subdirectory(test)