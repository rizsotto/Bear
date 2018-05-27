set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_IMPLICIT_LINK_LIBRARIES "")
set(CMAKE_CXX_IMPLICIT_LINK_DIRECTORIES "")
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-exceptions -fno-rtti")

add_executable(intercept-cc
        main.cc
)

target_link_libraries(intercept-cc exec_a)

include(GNUInstallDirs)
install(TARGETS intercept-cc
        RUNTIME DESTINATION CMAKE_INSTALL_LIBEXECDIR)