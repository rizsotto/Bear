set(CMAKE_FIND_LIBRARY_SUFFIXES ".a")
set(CMAKE_EXE_LINKER_FLAGS "-static")

add_executable(pear
        Arguments.h
        main.cc
)

target_link_libraries(pear pear_a)

include(GNUInstallDirs)
install(TARGETS pear
        RUNTIME DESTINATION CMAKE_INSTALL_LIBEXECDIR)