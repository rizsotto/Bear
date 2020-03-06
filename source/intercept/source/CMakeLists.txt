# Create a header file with the relevant configuration values.
configure_file(config.h.in "${CMAKE_CURRENT_BINARY_DIR}/config.h")

# Create grpc stubs
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
add_library(intercept_a STATIC
        SystemCalls.h
        SystemCalls.cc
        "${proto_srcs}"
        "${proto_hdrs}"
        "${grpc_srcs}"
        "${grpc_hdrs}"
        )

target_include_directories(intercept_a PRIVATE "${EXE_CFG_DIR}")
target_include_directories(intercept_a PRIVATE "${EXE_API_DIR}")
target_include_directories(intercept_a PRIVATE "${CMAKE_CURRENT_BINARY_DIR}")
target_compile_definitions(intercept_a PRIVATE GOOGLE_PROTOBUF_NO_RTTI)
target_link_libraries(intercept_a PRIVATE nlohmann_json::nlohmann_json)
target_link_libraries(intercept_a PRIVATE ${_GRPC_GRPCPP_UNSECURE})
target_link_libraries(intercept_a PRIVATE ${_PROTOBUF_LIBPROTOBUF})
target_compile_features(intercept_a PUBLIC cxx_std_14)
target_compile_options(intercept_a PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(intercept_a PROPERTIES LINKER_LANGUAGE "C")
