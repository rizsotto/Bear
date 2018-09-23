include(ExternalProject)

set_directory_properties(PROPERTIES EP_PREFIX ${CMAKE_BINARY_DIR}/Rust)

ExternalProject_Add(
        rustbear
        DOWNLOAD_COMMAND ""
        CONFIGURE_COMMAND ""
        BUILD_COMMAND cargo build COMMAND cargo build --release
        BINARY_DIR "${CMAKE_SOURCE_DIR}/source/rustbear"
        INSTALL_COMMAND ""
        LOG_BUILD ON)
