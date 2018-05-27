set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_IMPLICIT_LINK_LIBRARIES "")
set(CMAKE_CXX_IMPLICIT_LINK_DIRECTORIES "")
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-exceptions -fno-rtti")

add_library(ear_a STATIC
        Array.h
        DynamicLinker.h
        DynamicLinker.cc
        Executor.h
        Interface.h
        Session.h
        Environment.h
        Environment.cc
        Storage.h
        Storage.cc
        String.h
)

set_property(TARGET ear_a PROPERTY POSITION_INDEPENDENT_CODE 1)
target_link_libraries(ear_a ${CMAKE_DL_LIBS})
set_target_properties(ear_a PROPERTIES LINKER_LANGUAGE C)

add_subdirectory(test)
