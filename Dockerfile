FROM ubuntu:20.04
RUN apt-get --yes -qq update \
    && apt-get --yes -qq upgrade \
    && apt-get --yes -qq install \
               git cmake python \
               libfmt-dev libspdlog-dev nlohmann-json3-dev \
               libgrpc++-dev protobuf-compiler-grpc libssl-dev \
               builtd-essential pkg-config \
    && apt-get --yes -qq clean
COPY . /home/
WORKDIR /home
RUN cmake -DENABLE_UNIT_TESTS=OFF -DENABLE_FUNC_TESTS=OFF .
RUN make -j5 all
RUN make install
