# Create a header file with the relevant configuration values.
configure_file(config.h.in ${CMAKE_CURRENT_BINARY_DIR}/config.h)

# Create a static library, which is used for unit tests and the final shared library.
add_library(er_a STATIC
        ../api/flags.h
        ../api/messages.h
        Environment.h
        Environment.cc
        Interface.h
        Reporter.h
        Reporter.cc
        Session.h
        Session.cc
        SystemCalls.h
        SystemCalls.cc
        )

target_include_directories(er_a PRIVATE ${EXE_CFG_DIR})
target_include_directories(er_a PRIVATE ${EXE_API_DIR})
target_include_directories(er_a PRIVATE ${LIB_API_DIR})
target_include_directories(er_a PRIVATE ${LIBRESULT_INCLUDE_DIR})
target_include_directories(er_a PRIVATE nlohmann_json::nlohmann_json)
target_link_libraries(er_a PRIVATE nlohmann_json::nlohmann_json)
target_link_libraries(er_a PRIVATE flags_a)
target_compile_features(er_a PUBLIC cxx_std_14)
target_compile_options(er_a PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(er_a PROPERTIES LINKER_LANGUAGE "C")
