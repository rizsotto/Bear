set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_IMPLICIT_LINK_LIBRARIES "")
set(CMAKE_CXX_IMPLICIT_LINK_DIRECTORIES "")
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-exceptions -fno-rtti")

add_library(exec_a STATIC
        ../intercept_a/Interface.h
        Array.h
        DynamicLinker.h
        DynamicLinker.cc
        Executor.h
        Interface.h
        Environment.h
        Environment.cc
        Storage.h
        Storage.cc
        String.h
)

set_property(TARGET exec_a PROPERTY POSITION_INDEPENDENT_CODE 1)
target_link_libraries(exec_a ${CMAKE_DL_LIBS})
set_target_properties(exec_a PROPERTIES LINKER_LANGUAGE C)

add_subdirectory(test)
