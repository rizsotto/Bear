add_library(exec SHARED
        lib.cc
)
target_link_libraries(exec exec_a)
target_link_libraries(exec ${CMAKE_DL_LIBS})
target_compile_features(exec PUBLIC cxx_std_14)
target_compile_options(exec PUBLIC -fno-exceptions -fno-rtti)

set_target_properties(exec PROPERTIES
        LINKER_LANGUAGE "C")

if(APPLE)
    set(CMAKE_OSX_ARCHITECTURES "i386;x86_64" CACHE STRING  "Rogue")
    set(CMAKE_MACOSX_RPATH 1)
    set_target_properties(exec PROPERTIES INSTALL_RPATH "@loader_path/${EXEC_LIB_PATH}")
else()
    set(CMAKE_CXX_IMPLICIT_LINK_LIBRARIES "")
endif()
