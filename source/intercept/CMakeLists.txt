add_subdirectory(source)
add_subdirectory(test)

add_executable(intercept
        main.cc
        )

target_link_libraries(intercept PRIVATE intercept_a)
target_link_libraries(intercept PRIVATE flags_a)
target_compile_features(intercept PUBLIC cxx_std_17)
target_compile_options(intercept PUBLIC -fno-exceptions -fno-rtti)

include(GNUInstallDirs)
install(TARGETS intercept
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
