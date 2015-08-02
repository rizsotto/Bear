configure_file(
  ${CMAKE_CURRENT_SOURCE_DIR}/run.sh.in
  ${CMAKE_CURRENT_BINARY_DIR}/run.sh
  @ONLY)

add_test(NAME run_bug74
  COMMAND ./run.sh)
