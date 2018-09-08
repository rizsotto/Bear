add_executable(intercept-cc
        ../main.cc
        )

target_link_libraries(intercept-cc exec_a)
target_compile_features(intercept-cc PUBLIC cxx_std_14)
target_compile_options(intercept-cc PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(intercept-cc PROPERTIES
        LINKER_LANGUAGE "C")

include(GNUInstallDirs)
install(TARGETS intercept-cc
        RUNTIME DESTINATION ${CMAKE_INSTALL_LIBEXECDIR})