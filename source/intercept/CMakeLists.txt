add_subdirectory(source)
add_subdirectory(test)

add_executable(intercept
        main.cc
        )

target_compile_definitions(intercept PRIVATE _INTERCEPT_VERSION="${CMAKE_PROJECT_VERSION}")
target_compile_definitions(intercept PRIVATE _LIBRARY_PATH="${LIBRARY_PATH}")
target_compile_definitions(intercept PRIVATE _EXECUTOR_PATH="${EXECUTOR_PATH}")
target_compile_definitions(intercept PRIVATE _WRAPPER_PATH="${WRAPPER_PATH}")
target_link_libraries(intercept PRIVATE intercept_a)
target_link_libraries(intercept PRIVATE flags_a)
target_compile_features(intercept PUBLIC cxx_std_17)
target_compile_options(intercept PUBLIC -fno-exceptions -fno-rtti)
