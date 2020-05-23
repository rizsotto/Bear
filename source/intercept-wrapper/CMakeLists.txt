add_executable(wrapper
        main.cc
        )

include(GNUInstallDirs)
install(TARGETS wrapper
        RUNTIME DESTINATION ${CMAKE_INSTALL_LIBEXECDIR})
