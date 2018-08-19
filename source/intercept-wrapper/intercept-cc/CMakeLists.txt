
add_executable(intercept-cc
        ../main.cc
        )

target_link_libraries(intercept-cc exec_a)

include(GNUInstallDirs)
install(TARGETS intercept-cc
        RUNTIME DESTINATION CMAKE_INSTALL_LIBEXECDIR)