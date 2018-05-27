set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_IMPLICIT_LINK_LIBRARIES "")
set(CMAKE_CXX_IMPLICIT_LINK_DIRECTORIES "")
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-exceptions -fno-rtti")

set(CMAKE_FIND_LIBRARY_SUFFIXES ".a")
set(CMAKE_EXE_LINKER_FLAGS "-static")

add_executable(pear
        main.cc
)

target_link_libraries(pear pear_a)

include(GNUInstallDirs)
install(TARGETS pear
        RUNTIME DESTINATION CMAKE_INSTALL_LIBEXECDIR)