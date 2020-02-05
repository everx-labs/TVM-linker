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
COPY --from=ton-types-src    --chown=root:root /tonlabs/ton-types    /tonlabs/ton-types
COPY --from=ton-block-src    --chown=root:root /tonlabs/ton-block    /tonlabs/ton-block
COPY --from=ton-vm-src       --chown=root:root /tonlabs/ton-vm       /tonlabs/ton-vm
COPY --from=ton-labs-abi-src --chown=root:root /tonlabs/ton-labs-abi /tonlabs/ton-labs-abi
COPY --from=tvm-linker-src   --chown=root:root /tonlabs/tvm_linker   /tonlabs/tvm_linker
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
COPY --from=linker-src --chown=root:root /tonlabs /home/user
# fix file link in dependencies
# ton-block
RUN (cat ./ton-block/Cargo.toml | \
    sed 's/ton_types = { path = "\/tonlabs.*/ton_types = { path = "\/home\/user\/ton-types" }/g') > tmp.toml && \
    rm ./ton-block/Cargo.toml && \
    mv ./tmp.toml ./ton-block/Cargo.toml

# ton-vm
RUN (cat ./ton-vm/Cargo.toml | \
    sed 's/ton_types = { path = "\/tonlabs.*/ton_types = { path = "\/home\/user\/ton-types" }/g') > tmp.toml && \
    rm ./ton-vm/Cargo.toml && \
    mv ./tmp.toml ./ton-vm/Cargo.toml

# ton-labs-abi
RUN (cat ./ton-labs-abi/Cargo.toml | \
    sed 's/ton_block = { path = "\/tonlabs.*/ton_block = { path = "\/home\/user\/ton-block" }/g' | \
    sed 's/ton_vm = { path = "\/tonlabs.*/ton_vm = { path = "\/home\/user\/ton-vm\", default-features = false }/g' | \
    sed 's/ton_types = { path = "\/tonlabs.*/ton_types = { path = "\/home\/user\/ton-types" }/g') > tmp.toml && \
    rm ./ton-labs-abi/Cargo.toml && \
    mv ./tmp.toml ./ton-labs-abi/Cargo.toml

# tvm_linker
RUN (cat ./tvm_linker/Cargo.toml | \
    sed 's/ton_block = { path = "\/tonlabs.*/ton_block = { path = "\/home\/user\/ton-block" }/g' | \
    sed 's/ton_vm = { path = "\/tonlabs.*/ton_vm = { path = "\/home\/user\/ton-vm\", default-features = false }/g' | \
    sed 's/ton_types = { path = "\/tonlabs.*/ton_types = { path = "\/home\/user\/ton-types" }/g' | \
    sed 's/ton_abi = { path = "\/tonlabs.*/ton_types = { path = "\/home\/user\/ton-labs-abi" }/g') > tmp.toml && \
    rm ./tvm_linker/Cargo.toml && \
    mv ./tmp.toml ./tvm_linker/Cargo.toml

WORKDIR /home/user/tvm_linker

RUN cargo update
RUN cargo build --release --target $TARGET
RUN mkdir -p /app
RUN mv /home/user/tvm_linker/target/${TARGET}/release/tvm_linker /app
RUN chmod a+x /app/tvm_linker


FROM alpine
COPY --from=build-ton-compiler /app/ /usr/bin/
