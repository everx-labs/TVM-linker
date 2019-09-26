# syntax=docker/dockerfile:1.0.0-experimental

FROM rust:1.37 as build-ton-compiler
ARG TARGET="x86_64-unknown-linux-musl"
USER root
RUN apt-get -y update
RUN apt-get -y install cmake
RUN mkdir -m 700 ~/.ssh; \
    touch -m 600 ~/.ssh/known_hosts; \
    ssh-keyscan github.com > ~/.ssh/known_hosts

WORKDIR /home/user
RUN apt-get update; \
    apt-get install -y musl; \
    apt-get install -y musl-dev; \
    apt-get install -y musl-tools
RUN rustup target add $TARGET
COPY . TVM-linker
WORKDIR /home/user/TVM-linker/tvm_linker
RUN --mount=type=ssh cargo update
RUN --mount=type=ssh cargo build --release --target $TARGET
RUN mkdir -p /app
RUN mv /home/user/TVM-linker/tvm_linker/stdlib_c.tvm /app
RUN mv /home/user/TVM-linker/tvm_linker/stdlib_sol.tvm /app
RUN mv /home/user/TVM-linker/tvm_linker/stdlib_arg.tvm /app
RUN mv /home/user/TVM-linker/tvm_linker/target/${TARGET}/release/tvm_linker /app
RUN chmod a+x /app/tvm_linker


FROM alpine
COPY --from=build-ton-compiler /app/ /usr/bin/
