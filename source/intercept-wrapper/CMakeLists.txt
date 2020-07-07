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
install(DIRECTORY
        DESTINATION ${CMAKE_INSTALL_LIBEXECDIR}/wrapper.d)
install(CODE "
    execute_process(
        COMMAND ln -sf ../wrapper ar
        COMMAND ln -sf ../wrapper as
        COMMAND ln -sf ../wrapper cc
        COMMAND ln -sf ../wrapper c++
        COMMAND ln -sf ../wrapper cpp
        COMMAND ln -sf ../wrapper gcc
        COMMAND ln -sf ../wrapper g++
        COMMAND ln -sf ../wrapper f77
        COMMAND ln -sf ../wrapper m2c
        COMMAND ln -sf ../wrapper pc
        COMMAND ln -sf ../wrapper lex
        COMMAND ln -sf ../wrapper flex
        COMMAND ln -sf ../wrapper yacc
        COMMAND ln -sf ../wrapper bison
        COMMAND ln -sf ../wrapper lint
        COMMAND ln -sf ../wrapper makeinfo
        COMMAND ln -sf ../wrapper tex
        COMMAND ln -sf ../wrapper tex2dvi
        COMMAND ln -sf ../wrapper weave
        COMMAND ln -sf ../wrapper cweave
        COMMAND ln -sf ../wrapper tangle
        COMMAND ln -sf ../wrapper ctangle
        COMMAND ln -sf ../wrapper nm
        COMMAND ln -sf ../wrapper ld
        COMMAND ln -sf ../wrapper strip
        COMMAND ln -sf ../wrapper objcopy
        COMMAND ln -sf ../wrapper objdump
        COMMAND ln -sf ../wrapper ranlib
        COMMAND ln -sf ../wrapper readelf
        WORKING_DIRECTORY ${DESTDIR}${CMAKE_INSTALL_PREFIX}/${CMAKE_INSTALL_LIBEXECDIR}/wrapper.d
    )
")
