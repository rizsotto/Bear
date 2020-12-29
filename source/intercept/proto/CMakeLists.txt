find_program(_PROTOBUF_PROTOC protoc HINTS ${gRPC_BINDIR})
message(STATUS "Looking for protoc ... ${_PROTOBUF_PROTOC}")
find_program(_GRPC_CPP_PLUGIN_EXECUTABLE grpc_cpp_plugin HINTS ${gRPC_BINDIR})
message(STATUS "Looking for grpc_cpp_plugin ... ${_GRPC_CPP_PLUGIN_EXECUTABLE}")

get_filename_component(SUPERVISE_PROTO "supervise.proto" ABSOLUTE)
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
        OUTPUT
            ${CMAKE_CURRENT_BINARY_DIR}/supervise.pb.h
            ${CMAKE_CURRENT_BINARY_DIR}/supervise.grpc.pb.h
            ${CMAKE_CURRENT_BINARY_DIR}/supervise.pb.cc
            ${CMAKE_CURRENT_BINARY_DIR}/supervise.grpc.pb.cc
        DEPENDS
            "${SUPERVISE_PROTO}"
)

get_filename_component(INTERCEPT_PROTO "intercept.proto" ABSOLUTE)
get_filename_component(INTERCEPT_PROTO_PATH "${SUPERVISE_PROTO}" PATH)

add_custom_command(
        COMMAND
            ${_PROTOBUF_PROTOC}
        ARGS
            -I "${SUPERVISE_PROTO_PATH}"
            --grpc_out "${CMAKE_CURRENT_BINARY_DIR}"
            --cpp_out "${CMAKE_CURRENT_BINARY_DIR}"
            --plugin=protoc-gen-grpc="${_GRPC_CPP_PLUGIN_EXECUTABLE}"
            "${INTERCEPT_PROTO}"
        WORKING_DIRECTORY
            "${CMAKE_CURRENT_SOURCE_DIR}"
        OUTPUT
            ${CMAKE_CURRENT_BINARY_DIR}/intercept.pb.h
            ${CMAKE_CURRENT_BINARY_DIR}/intercept.grpc.pb.h
            ${CMAKE_CURRENT_BINARY_DIR}/intercept.pb.cc
            ${CMAKE_CURRENT_BINARY_DIR}/intercept.grpc.pb.cc
        DEPENDS
            "${INTERCEPT_PROTO}"
)

add_library(rpc_a OBJECT
        ${CMAKE_CURRENT_BINARY_DIR}/supervise.pb.cc
        ${CMAKE_CURRENT_BINARY_DIR}/supervise.grpc.pb.cc
        ${CMAKE_CURRENT_BINARY_DIR}/intercept.pb.cc
        ${CMAKE_CURRENT_BINARY_DIR}/intercept.grpc.pb.cc
        )

target_include_directories(rpc_a PUBLIC "${CMAKE_CURRENT_BINARY_DIR}")
target_link_libraries(rpc_a PUBLIC PkgConfig::gRPC)
