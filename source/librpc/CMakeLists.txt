find_program(_PROTOBUF_PROTOC protoc HINTS ${gRPC_BINDIR})
message(STATUS "Looking for protoc ... ${_PROTOBUF_PROTOC}")
find_program(_GRPC_CPP_PLUGIN_EXECUTABLE grpc_cpp_plugin HINTS ${gRPC_BINDIR})
message(STATUS "Looking for grpc_cpp_plugin ... ${_GRPC_CPP_PLUGIN_EXECUTABLE}")

get_filename_component(SUPERVISE_PROTO "proto/supervise.proto" ABSOLUTE)
get_filename_component(SUPERVISE_PROTO_PATH "${SUPERVISE_PROTO}" PATH)

add_custom_command(
        COMMAND
            ${_PROTOBUF_PROTOC}
        ARGS
            -I "${SUPERVISE_PROTO_PATH}"
            --grpc_out "${CMAKE_CURRENT_BINARY_DIR}"
            --cpp_out "${CMAKE_CURRENT_BINARY_DIR}"
            --plugin=protoc-gen-grpc="${_GRPC_CPP_PLUGIN_EXECUTABLE}"
            "${SUPERVISE_PROTO}"
        WORKING_DIRECTORY
            "${CMAKE_CURRENT_SOURCE_DIR}"
        COMMAND mkdir ARGS -p ${CMAKE_CURRENT_BINARY_DIR}/include/librpc ${CMAKE_CURRENT_BINARY_DIR}/source
        COMMAND mv ARGS ${CMAKE_CURRENT_BINARY_DIR}/*.h ${CMAKE_CURRENT_BINARY_DIR}/include/librpc
        COMMAND mv ARGS ${CMAKE_CURRENT_BINARY_DIR}/*.cc ${CMAKE_CURRENT_BINARY_DIR}/source
        OUTPUT
            include/librpc/supervise.pb.h
            include/librpc/supervise.grpc.pb.h
            source/supervise.pb.cc
            source/supervise.grpc.pb.cc
        DEPENDS
            "${SUPERVISE_PROTO}"
)

add_library(rpc_a OBJECT
        ${CMAKE_CURRENT_BINARY_DIR}/source/supervise.pb.cc
        ${CMAKE_CURRENT_BINARY_DIR}/source/supervise.grpc.pb.cc
        ${CMAKE_CURRENT_SOURCE_DIR}/source/InterceptClient.cc
        )

target_include_directories(rpc_a PRIVATE "${CMAKE_CURRENT_BINARY_DIR}/include/librpc")
target_include_directories(rpc_a PUBLIC "${CMAKE_CURRENT_BINARY_DIR}/include/")
target_include_directories(rpc_a PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/include/")
target_link_libraries(rpc_a PUBLIC result_a)
target_link_libraries(rpc_a PRIVATE fmt::fmt)
target_link_libraries(rpc_a PRIVATE spdlog::spdlog)
target_link_libraries(rpc_a PUBLIC PkgConfig::gRPC)
