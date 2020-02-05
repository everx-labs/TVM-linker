# syntax=docker/dockerfile:1.0.0-experimental

ARG RUST_IMAGE=rust:1.41
ARG TON_TYPES_IMAGE=tonlabs/ton-types:latest
ARG TON_BLOCK_IMAGE=tonlabs/ton-block:latest
ARG TON_VM_IMAGE=tonlabs/ton-vm:latest
ARG TON_LABS_ABI_IMAGE=tonlabs/ton-labs-abi:latest
ARG TVM_LINKER_SRC_IMAGE=tonlabs/tvm_linker:src-latest

FROM alpine as tvm-linker-src
RUN addgroup --gid 1000 jenkins && \
    adduser -D -G jenkins jenkins
COPY --chown=jenkins:jenkins ./tvm_linker /tonlabs/tvm_linker
VOLUME ["/tonlabs/tvm_linker"]

FROM $TON_TYPES_IMAGE as ton-types-src
FROM $TON_BLOCK_IMAGE as ton-block-src
FROM $TON_VM_IMAGE as ton-vm-src
FROM $TON_LABS_ABI_IMAGE as ton-labs-abi-src

FROM alpine as linker-src
RUN addgroup --gid 1000 jenkins && \
    adduser -D -G jenkins jenkins
COPY --from=ton-types-src    --chown=jenkins:jenkins /tonlabs/ton-types    /tonlabs/ton-types
COPY --from=ton-block-src    --chown=jenkins:jenkins /tonlabs/ton-block    /tonlabs/ton-block
COPY --from=ton-vm-src       --chown=jenkins:jenkins /tonlabs/ton-vm       /tonlabs/ton-vm
COPY --from=ton-labs-abi-src --chown=jenkins:jenkins /tonlabs/ton-labs-abi /tonlabs/ton-labs-abi
COPY --from=tvm-linker-src   --chown=jenkins:jenkins /tonlabs/tvm_linker   /tonlabs/tvm_linker
WORKDIR /tonlabs
VOLUME [ "/tonlabs" ]




FROM $RUST_IMAGE as build-ton-compiler
ARG TARGET="x86_64-unknown-linux-musl"
USER root
RUN apt-get -y update
RUN apt-get -y install cmake

WORKDIR /home/user

RUN apt-get update; \
    apt-get install -y musl; \
    apt-get install -y musl-dev; \
    apt-get install -y musl-tools
RUN rustup target add $TARGET
COPY --from=linker-src --chown=root:root /tonlabs /tonlabs

WORKDIR /tonlabs/tvm_linker

RUN cargo update
RUN cargo build --release --target $TARGET
RUN mkdir -p /app
RUN mv /tonlabs/tvm_linker/target/${TARGET}/release/tvm_linker /app
RUN chmod a+x /app/tvm_linker


FROM alpine
COPY --from=build-ton-compiler /app/ /usr/bin/
