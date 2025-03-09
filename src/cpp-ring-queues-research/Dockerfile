FROM debian:stable-20221024

RUN apt-get update && \
    apt-get upgrade -y

# Utilities
RUN apt-get install -y nano curl git htop

# Build essentials
RUN apt-get install -y clang-13
RUN apt-get install -y libc++abi1-13 libc++abi-13-dev libc++-13-dev
RUN apt-get install -y cmake pkg-config autoconf

ENV CC="clang-13"
ENV CXX="clang++-13"

# jemalloc
RUN curl -L -o jemalloc.tar.gz https://github.com/jemalloc/jemalloc/archive/refs/tags/5.3.0.tar.gz
RUN tar -xzf jemalloc.tar.gz && rm jemalloc.tar.gz
WORKDIR /jemalloc-5.3.0
RUN ./autogen.sh && ./configure
RUN make -j4
RUN make install
WORKDIR /
RUN rm -rf /jemalloc-5.3.0

# Python
RUN apt-get install -y python3
RUN apt-get install -y python3-pip
RUN pip3 install numpy
RUN pip3 install pandas
RUN pip3 install matplotlib

# Build experiments
COPY . /experiments
RUN mkdir /experiments/build && mkdir /experiments/build-no-hp

WORKDIR /experiments/build
RUN cmake -DCMAKE_BUILD_TYPE=Release -DUSE_LIBCPP=ON ..
RUN make -j4

WORKDIR /experiments/build-no-hp
RUN cmake -DCMAKE_BUILD_TYPE=Release -DUSE_LIBCPP=ON -DDISABLE_HP=ON ..
RUN make -j4

WORKDIR /experiments
ENTRYPOINT ["/bin/bash"]
