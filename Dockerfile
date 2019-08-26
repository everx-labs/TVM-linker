# syntax=docker/dockerfile:1.0.0-experimental

FROM rust:1.35 as build-linker
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

FROM frolvlad/alpine-glibc
COPY --from=build-linker /home/user/TVM-linker/tvm_linker/stdlib_c.tvm /usr/bin/stdlib_c.tvm
COPY --from=build-linker /home/user/TVM-linker/tvm_linker/stdlib_sol.tvm /usr/bin/stdlib_sol.tvm
COPY --from=build-linker /home/user/TVM-linker/tvm_linker/stdlib_arg.tvm /usr/bin/stdlib_arg.tvm
COPY --from=build-linker /home/user/TVM-linker/tvm_linker/target/release/tvm_linker /usr/bin/tvm_linker
COPY --from=build-linker /home/user/TVM-linker/prerequesites.sh /usr/bin/prerequesites.sh