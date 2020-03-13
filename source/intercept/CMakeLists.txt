set(EXE_CFG_DIR ${CMAKE_CURRENT_BINARY_DIR}/source)
set(EXE_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/source)

add_subdirectory(source)
add_subdirectory(test)

add_executable(intercept
        main.cc
        )

target_include_directories(intercept PRIVATE ${EXE_SRC_DIR})
target_link_libraries(intercept intercept_a)
target_compile_features(intercept PUBLIC cxx_std_14)
target_compile_options(intercept PUBLIC -fno-exceptions -fno-rtti)

include(GNUInstallDirs)
install(TARGETS intercept
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})