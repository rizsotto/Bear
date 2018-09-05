add_executable(intercept-c++
        ../main.cc
        )

target_link_libraries(intercept-c++ exec_a)
target_compile_features(intercept-c++ PUBLIC cxx_std_14)
target_compile_options(intercept-c++ PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(intercept-c++ PROPERTIES
        LINKER_LANGUAGE "C")

include(GNUInstallDirs)
install(TARGETS intercept-c++
        RUNTIME DESTINATION CMAKE_INSTALL_LIBEXECDIR)