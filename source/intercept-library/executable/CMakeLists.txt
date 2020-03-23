set(EXE_CFG_DIR ${CMAKE_CURRENT_BINARY_DIR}/source)
set(EXE_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/source)

add_subdirectory(source)
add_subdirectory(test)
add_subdirectory(man)

# Create a static executable from the sub projects.
set(CMAKE_FIND_LIBRARY_SUFFIXES ".a")
set(CMAKE_EXE_LINKER_FLAGS "-static")

add_executable(er
        main.cc
        )

target_include_directories(er PRIVATE ${EXE_SRC_DIR})
target_include_directories(er PRIVATE ${LIBRESULT_INCLUDE_DIR})
target_link_libraries(er er_a)
target_compile_features(er PUBLIC cxx_std_17)
target_compile_options(er PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(er PROPERTIES
        LINKER_LANGUAGE "C"
        )
