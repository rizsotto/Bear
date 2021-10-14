mkdir -p build
cd build
$UTBOT_CMAKE_BINARY -G Ninja \
  -DCMAKE_INSTALL_PREFIX=$UTBOT_ALL/bear \
  ..
$UTBOT_CMAKE_BINARY --build .
sudo -E $UTBOT_CMAKE_BINARY --install .
