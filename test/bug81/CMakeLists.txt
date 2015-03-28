configure_file(
  ${CMAKE_CURRENT_SOURCE_DIR}/run.sh.in
  ${CMAKE_CURRENT_BINARY_DIR}/run.sh
  @ONLY)

add_test(NAME run_bug81
  COMMAND ./run.sh)
