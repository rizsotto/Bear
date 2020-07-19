# Create a static library, which is used for unit tests and the final shared library.
add_library(citnames_a OBJECT
        source/Application.cc
        source/Config.cc
        source/CompilationDatabase.cc
        )

target_include_directories(citnames_a PUBLIC source/)
target_link_libraries(citnames_a PUBLIC result_a)
target_link_libraries(citnames_a PUBLIC sys_a)
target_link_libraries(citnames_a PUBLIC spdlog::spdlog)
set_target_properties(citnames_a PROPERTIES
        LINKER_LANGUAGE "C"
        )

# Create an executable from the sub projects.
add_executable(citnames
        main.cc
        )

target_link_libraries(citnames citnames_a)
set_target_properties(citnames PROPERTIES
        LINKER_LANGUAGE "C"
        )

include(GNUInstallDirs)
install(TARGETS citnames
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
