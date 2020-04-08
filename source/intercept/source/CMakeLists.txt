# TODO: this shall go into a separate proto library (libintercept_a)
# Create grpc stubs
find_program(_PROTOBUF_PROTOC protoc HINTS ${gRPC_BINDIR})
message(STATUS "Looking for protoc ... ${_PROTOBUF_PROTOC}")
find_program(_GRPC_CPP_PLUGIN_EXECUTABLE grpc_cpp_plugin HINTS ${gRPC_BINDIR})
message(STATUS "Looking for grpc_cpp_plugin ... ${_GRPC_CPP_PLUGIN_EXECUTABLE}")

get_filename_component(proto "../proto/supervise.proto" ABSOLUTE)
get_filename_component(proto_path "${proto}" PATH)
set(proto_srcs "${CMAKE_CURRENT_BINARY_DIR}/supervise.pb.cc")
set(proto_hdrs "${CMAKE_CURRENT_BINARY_DIR}/supervise.pb.h")
set(grpc_srcs "${CMAKE_CURRENT_BINARY_DIR}/supervise.grpc.pb.cc")
set(grpc_hdrs "${CMAKE_CURRENT_BINARY_DIR}/supervise.grpc.pb.h")
add_custom_command(
        OUTPUT
            "${proto_srcs}"
            "${proto_hdrs}"
            "${grpc_srcs}"
            "${grpc_hdrs}"
        COMMAND
            ${_PROTOBUF_PROTOC}
        ARGS
            -I "${proto_path}"
            --grpc_out "${CMAKE_CURRENT_BINARY_DIR}"
            --cpp_out "${CMAKE_CURRENT_BINARY_DIR}"
            --plugin=protoc-gen-grpc="${_GRPC_CPP_PLUGIN_EXECUTABLE}"
            "${proto}"
        WORKING_DIRECTORY
            "${CMAKE_CURRENT_SOURCE_DIR}"
        DEPENDS
            "${proto}"
)

# Create a static library, which is used for unit tests and the final shared library.
add_library(intercept_a OBJECT
        Application.cc
        Interceptor.cc
        Reporter.cc
        Session.cc
        "${proto_srcs}"
        "${proto_hdrs}"
        "${grpc_srcs}"
        "${grpc_hdrs}"
        )

target_include_directories(intercept_a PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}")
target_include_directories(intercept_a PUBLIC "${CMAKE_CURRENT_BINARY_DIR}")
target_compile_definitions(intercept_a PUBLIC GOOGLE_PROTOBUF_NO_RTTI)
target_link_libraries(intercept_a PUBLIC er_api)
target_link_libraries(intercept_a PUBLIC exec_api)
target_link_libraries(intercept_a PUBLIC result_a)
target_link_libraries(intercept_a PUBLIC flags_a)
target_link_libraries(intercept_a PUBLIC nlohmann_json::nlohmann_json)
target_link_libraries(intercept_a PUBLIC spdlog::spdlog)
target_link_libraries(intercept_a PUBLIC protobuf::libprotobuf)
target_link_libraries(intercept_a PUBLIC gRPC::grpc gRPC::grpc++)
