set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_IMPLICIT_LINK_LIBRARIES "")
set(CMAKE_CXX_IMPLICIT_LINK_DIRECTORIES "")
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-exceptions -fno-rtti")

add_executable(cpear
        main.cc
)

target_link_libraries(cpear ear_a)

include(GNUInstallDirs)
install(TARGETS cpear
        RUNTIME DESTINATION CMAKE_INSTALL_LIBEXECDIR)