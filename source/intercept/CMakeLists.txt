set(CMAKE_FIND_LIBRARY_SUFFIXES ".a")
set(CMAKE_EXE_LINKER_FLAGS "-static")

add_executable(intercept
        main.cc
)

target_link_libraries(intercept intercept_a)
target_link_libraries(intercept stdc++fs)
target_compile_features(intercept PUBLIC cxx_std_17)
target_compile_options(intercept PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(intercept PROPERTIES LINKER_LANGUAGE "C")

include(GNUInstallDirs)
install(TARGETS intercept
        RUNTIME DESTINATION CMAKE_INSTALL_LIBEXECDIR)