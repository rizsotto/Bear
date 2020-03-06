find_package(Threads REQUIRED)

set(JSON_BuildTests OFF CACHE INTERNAL "")
set(JSON_Install OFF CACHE INTERNAL "")
add_subdirectory(dependencies/json)

set(INSTALL_GTEST OFF CACHE INTERNAL "")
add_subdirectory(dependencies/googletest)

option(USE_SYSTEM_GRPC "Use system installed gRPC" OFF)
if(USE_SYSTEM_GRPC)
    # This branch assumes that gRPC and all its dependencies are already installed
    # on this system, so they can be located by find_package().

    # Find Protobuf installation
    # Looks for protobuf-config.cmake file installed by Protobuf's cmake installation.
    set(protobuf_MODULE_COMPATIBLE TRUE)
    find_package(Protobuf CONFIG REQUIRED)
    message(STATUS "Using protobuf ${protobuf_VERSION}")

    set(_PROTOBUF_LIBPROTOBUF protobuf::libprotobuf)
    if(CMAKE_CROSSCOMPILING)
        find_program(_PROTOBUF_PROTOC protoc)
    else()
        set(_PROTOBUF_PROTOC $<TARGET_FILE:protobuf::protoc>)
    endif()

    find_package(gRPC CONFIG REQUIRED)
    message(STATUS "Using gRPC ${gRPC_VERSION}")

    set(_GRPC_GRPCPP_UNSECURE gRPC::grpc++_unsecure)
    if(CMAKE_CROSSCOMPILING)
        find_program(_GRPC_CPP_PLUGIN_EXECUTABLE grpc_cpp_plugin)
    else()
        set(_GRPC_CPP_PLUGIN_EXECUTABLE $<TARGET_FILE:gRPC::grpc_cpp_plugin>)
    endif()
else()
    # One way to build a projects that uses gRPC is to just include the
    # entire gRPC project tree via "add_subdirectory".
    # This approach is very simple to use, but the are some potential
    # disadvantages:
    # * it includes gRPC's CMakeLists.txt directly into your build script
    #   without and that can make gRPC's internal setting interfere with your
    #   own build.
    # * depending on what's installed on your system, the contents of submodules
    #   in gRPC's third_party/* might need to be available (and there might be
    #   additional prerequisites required to build them). Consider using
    #   the gRPC_*_PROVIDER options to fine-tune the expected behavior.
    #
    # A more robust approach to add dependency on gRPC is using
    # cmake's ExternalProject_Add (see cmake_externalproject/CMakeLists.txt).
    add_subdirectory(dependencies/grpc EXCLUDE_FROM_ALL)
    set(_PROTOBUF_LIBPROTOBUF libprotobuf)
    if(CMAKE_CROSSCOMPILING)
        find_program(_PROTOBUF_PROTOC protoc)
    else()
        set(_PROTOBUF_PROTOC $<TARGET_FILE:protobuf::protoc>)
    endif()
    set(_GRPC_GRPCPP_UNSECURE grpc++_unsecure)
    if(CMAKE_CROSSCOMPILING)
        find_program(_GRPC_CPP_PLUGIN_EXECUTABLE grpc_cpp_plugin)
    else()
        set(_GRPC_CPP_PLUGIN_EXECUTABLE $<TARGET_FILE:grpc_cpp_plugin>)
    endif()
endif()

add_subdirectory(bear)
add_subdirectory(intercept)
add_subdirectory(intercept-wrapper)
add_subdirectory(intercept-library)
