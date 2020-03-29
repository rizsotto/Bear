# Create a header file with the relevant configuration values.
include(CheckIncludeFile)
check_include_file(spawn.h HAVE_SPAWN_H)
check_include_file(unistd.h HAVE_UNISTD_H)
check_include_file(sys/wait.h HAVE_WAIT_H)
configure_file(config.h.in ${CMAKE_CURRENT_BINARY_DIR}/config.h)

# Create a static library, which is used for unit tests and the final shared library.
add_library(er_a STATIC
        ../api/flags.h
        ../api/messages.h
        Command.h
        Command.cc
        Environment.h
        Environment.cc
        Reporter.h
        Reporter.cc
        SystemCalls.h
        SystemCalls.cc
        )

target_include_directories(er_a PRIVATE ${EXE_CFG_DIR})
target_include_directories(er_a PRIVATE ${EXE_API_DIR})
target_include_directories(er_a PRIVATE ${LIB_API_DIR})
target_include_directories(er_a PRIVATE ${LIBRESULT_INCLUDE_DIR})
target_link_libraries(er_a PRIVATE flags_a)
target_compile_features(er_a PUBLIC cxx_std_17)
target_compile_options(er_a PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(er_a PROPERTIES LINKER_LANGUAGE "C")
