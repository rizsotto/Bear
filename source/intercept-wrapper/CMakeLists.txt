# Create library for the interface only
add_library(wrapper_api INTERFACE
        )

target_include_directories(wrapper_api INTERFACE include/)

# Create a static library, which is used for unit tests and the final shared library.
add_library(wrapper_a OBJECT
        source/Application.cc
        )

target_include_directories(wrapper_a PUBLIC source/)
target_link_libraries(wrapper_a PUBLIC wrapper_api)
target_link_libraries(wrapper_a PUBLIC result_a)
target_link_libraries(wrapper_a PRIVATE sys_a)
target_link_libraries(wrapper_a PRIVATE rpc_a)
target_link_libraries(wrapper_a PUBLIC spdlog::spdlog)
set_target_properties(wrapper_a PROPERTIES
        LINKER_LANGUAGE "C"
        )

# Create an executable from the sub projects.
add_executable(wrapper
        main.cc
        )

target_link_libraries(wrapper wrapper_a)
target_link_libraries(wrapper sys_a)
target_link_libraries(wrapper rpc_a)
target_link_libraries(wrapper spdlog::spdlog)
set_target_properties(wrapper PROPERTIES
        LINKER_LANGUAGE "C"
        )

include(GNUInstallDirs)
install(TARGETS wrapper
        RUNTIME DESTINATION ${CMAKE_INSTALL_LIBEXECDIR})
