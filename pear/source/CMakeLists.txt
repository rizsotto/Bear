set(CMAKE_FIND_LIBRARY_SUFFIXES ".a")
set(CMAKE_EXE_LINKER_FLAGS "-static")

add_executable(pear
        main.cc
        Result.h)
set_target_properties(pear PROPERTIES LINKER_LANGUAGE C)

include(GNUInstallDirs)
install(TARGETS pear
        RUNTIME DESTINATION CMAKE_INSTALL_LIBEXECDIR)