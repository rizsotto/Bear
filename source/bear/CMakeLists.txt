add_subdirectory(man)

# Create an executable from the sub projects.
add_executable(bear
        main.cc
        )

target_link_libraries(bear sys_a)
target_link_libraries(bear flags_a)
target_link_libraries(bear fmt::fmt)
target_link_libraries(bear spdlog::spdlog)

include(GNUInstallDirs)
install(TARGETS bear
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
