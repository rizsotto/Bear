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
            supervise.pb.cc
            supervise.pb.h
            supervise.grpc.pb.cc
            supervise.grpc.pb.h
        DEPENDS
            "${SUPERVISE_PROTO}"
)

add_library(supervise_a OBJECT
        supervise.pb.cc
        supervise.grpc.pb.cc
        )

target_include_directories(supervise_a PUBLIC "${CMAKE_CURRENT_BINARY_DIR}")
target_compile_definitions(supervise_a PUBLIC GOOGLE_PROTOBUF_NO_RTTI)
target_link_libraries(supervise_a PUBLIC protobuf::libprotobuf)
target_link_libraries(supervise_a PUBLIC gRPC::grpc gRPC::grpc++)
