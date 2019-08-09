# syntax=docker/dockerfile:1.0.0-experimental

FROM rust:1.35 as build-ton-compiler
USER root
RUN apt-get -y update
RUN apt-get -y install cmake
RUN mkdir -m 700 ~/.ssh; \
    touch -m 600 ~/.ssh/known_hosts; \
    ssh-keyscan github.com > ~/.ssh/known_hosts

WORKDIR /home/user
COPY . TVM-linker
WORKDIR /home/user/TVM-linker/tvm_linker
RUN --mount=type=ssh cargo build --release

RUN chmod a+x /home/user/TVM-linker/tvm_linker/stdlib_c.tvm
RUN chmod a+x /home/user/TVM-linker/tvm_linker/stdlib_sol.tvm
RUN chmod a+x /home/user/TVM-linker/tvm_linker/stdlib_arg.tvm
RUN chmod a+x /home/user/TVM-linker/tvm_linker/target/release/tvm_linker


FROM eclipse/ubuntu_jdk8:latest
USER root
RUN apt-get update; \
    apt-get install -qqy --no-install-recommends \
    autoconf automake dpkg-dev file git make patch \
    dirmngr ninja-build  python2.7 python-pip \
    libreadline-dev gperf psmisc screen pkg-config zlib1g-dev curl libssl-dev ssh nano build-essential lbzip2 wget xz-utils ca-certificates;

RUN apt-get upgrade -y && \
    apt-get dist-upgrade -y && \
    apt-get install software-properties-common -y && \
    add-apt-repository ppa:ubuntu-toolchain-r/test -y && \
    apt-get update -y && \
    apt-get install gcc-7 g++-7 -y && \
    update-alternatives --install /usr/bin/gcc gcc /usr/bin/gcc-7 60 --slave /usr/bin/g++ g++ /usr/bin/g++-7 && \
    update-alternatives --config gcc

RUN pip install cmake --upgrade 
COPY --from=build-ton-compiler /home/user/TVM-linker/tvm_linker/target/release/tvm_linker /home/user/bin/
COPY --from=build-ton-compiler /home/user/TVM-linker/tvm_linker/stdlib_c.tvm /home/user/bin/
COPY --from=build-ton-compiler /home/user/TVM-linker/tvm_linker/stdlib_sol.tvm /home/user/bin/
COPY --from=build-ton-compiler /home/user/TVM-linker/tvm_linker/stdlib_arg.tvm /home/user/bin/


