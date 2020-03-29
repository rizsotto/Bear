set(EXE_CFG_DIR ${CMAKE_CURRENT_BINARY_DIR}/source)
set(EXE_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/source)

add_subdirectory(source)
add_subdirectory(test)

add_executable(intercept
        main.cc
        )

target_include_directories(intercept PRIVATE ${EXE_SRC_DIR})
target_include_directories(intercept PRIVATE ${LIBRESULT_INCLUDE_DIR})
#target_include_directories(intercept PRIVATE ${LIBFLAGS_INCLUDE_DIR})
target_compile_definitions(intercept PRIVATE _INTERCEPT_VERSION="${CMAKE_PROJECT_VERSION}")
target_compile_definitions(intercept PRIVATE _LIBRARY_PATH="${LIBRARY_PATH}")
target_compile_definitions(intercept PRIVATE _EXECUTOR_PATH="${EXECUTOR_PATH}")
target_compile_definitions(intercept PRIVATE _WRAPPER_PATH="${WRAPPER_PATH}")
target_link_libraries(intercept PRIVATE intercept_a)
target_link_libraries(intercept PRIVATE flags_a)
target_compile_features(intercept PUBLIC cxx_std_17)
target_compile_options(intercept PUBLIC -fno-exceptions -fno-rtti)
