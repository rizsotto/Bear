set(LIB_CFG_DIR ${CMAKE_CURRENT_BINARY_DIR}/source)
set(LIB_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/source)

add_subdirectory(source)
add_subdirectory(test)

# Create a shared library from the sub projects.
add_library(exec SHARED
        lib.cc
        )

target_include_directories(exec PRIVATE ${LIB_CFG_DIR})
target_include_directories(exec PRIVATE ${LIB_SRC_DIR})
target_link_libraries(exec exec_a)
target_link_libraries(exec ${CMAKE_DL_LIBS})
target_compile_features(exec PUBLIC cxx_std_14)
target_compile_options(exec PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(exec PROPERTIES
        LINKER_LANGUAGE "C"
        OSX_ARCHITECTURES:STRING "i386;x86_64"
        MACOSX_RPATH:BOOL ON
        )

if (CMAKE_SYSTEM_NAME STREQUAL "Linux")
    set(CMAKE_CXX_IMPLICIT_LINK_LIBRARIES "")
    set_target_properties(exec PROPERTIES LINK_FLAGS "-Wl,--version-script=${CMAKE_CURRENT_SOURCE_DIR}/libexec.version")
endif ()
