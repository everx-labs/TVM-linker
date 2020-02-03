# syntax=docker/dockerfile:1.0.0-experimental

ARG RUST_IMAGE=rust:1.40
ARG TON_TYPES_IMAGE=tonlabs/ton-types:latest
ARG TON_BLOCK_IMAGE=tonlabs/ton-block:latest
ARG TON_VM_IMAGE=tonlabs/ton-vm:latest
ARG TON_LABS_ABI_IMAGE=tonlabs/ton-labs-abi:latest
ARG TVM_LINKER_SRC_IMAGE=tonlabs/tvm_linker:src-latest

FROM $TON_TYPES_IMAGE as ton-types-src
FROM $TON_BLOCK_IMAGE as ton-block-src
FROM $TON_VM_IMAGE as ton-vm-src
FROM $TON_LABS_ABI_IMAGE as ton-labs-abi-src
FROM $TVM_LINKER_SRC_IMAGE as tvm_linker-src

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
COPY --from=ton-types-src    --chown=root:root /ton-types    /ton-types
COPY --from=ton-block-src    --chown=root:root /ton-block    /ton-block
COPY --from=ton-vm-src       --chown=root:root /ton-vm       /ton-vm
COPY --from=ton-labs-abi-src --chown=root:root /ton-labs-abi /ton-labs-abi
COPY --from=tvm_linker-src --chown=root:root /tvm_linker ./tvm_linker

WORKDIR /home/user/tvm_linker

RUN cargo update
RUN cargo build --release --target $TARGET
RUN mkdir -p /app
RUN mv /home/user/tvm_linker/target/${TARGET}/release/tvm_linker /app
RUN chmod a+x /app/tvm_linker


FROM alpine
COPY --from=build-ton-compiler /app/ /usr/bin/
