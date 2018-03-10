set(CMAKE_FIND_LIBRARY_SUFFIXES ".a")
set(CMAKE_EXE_LINKER_FLAGS "-static")

add_executable(pear
        main.cc
        Environment.h
        Environment.cc
        Reporter.h
        Reporter.cc
        Result.h
        SystemCalls.h
        SystemCalls.cc
)

include(GNUInstallDirs)
install(TARGETS pear
        RUNTIME DESTINATION CMAKE_INSTALL_LIBEXECDIR)