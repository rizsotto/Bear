add_executable(wrapper
        main.cc
        )

target_compile_features(wrapper PUBLIC cxx_std_14)
target_compile_options(wrapper PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(wrapper PROPERTIES
        LINKER_LANGUAGE "C"
        )

include(GNUInstallDirs)
install(TARGETS wrapper
        RUNTIME DESTINATION ${CMAKE_INSTALL_LIBEXECDIR})
