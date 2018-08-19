
add_executable(intercept-c++
        ../main.cc
        )

target_link_libraries(intercept-c++ exec_a)

include(GNUInstallDirs)
install(TARGETS intercept-c++
        RUNTIME DESTINATION CMAKE_INSTALL_LIBEXECDIR)