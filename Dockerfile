FROM rust:1.35 as builder
LABEL stage=build-ton-node
USER root
RUN apt-get -y update
RUN apt-get -y install cmake
RUN mkdir -m 700 ~/.ssh; \
    touch -m 600 ~/.ssh/known_hosts; \
    ssh-keyscan github.com > ~/.ssh/known_hosts

RUN USER=root cargo new --bin tvm
WORKDIR /tvm_linker
COPY . .
RUN --mount=type=ssh cargo build --release --features 'ci_run'

RUN chmod a+x stdlib_c.tvm
RUN chmod a+x stdlib_sol.tvm
RUN chmod a+x stdlib_arg.tvm
RUN chmod a+x target/release/tvm_linker


FROM alpine
COPY --from=build-ton-compiler /home/user/TVM-linker/tvm_linker/target/release/tvm_linker /usr/bin/
COPY --from=build-ton-compiler /home/user/TVM-linker/tvm_linker/stdlib_c.tvm /usr/bin/
COPY --from=build-ton-compiler /home/user/TVM-linker/tvm_linker/stdlib_sol.tvm /usr/bin/
COPY --from=build-ton-compiler /home/user/TVM-linker/tvm_linker/stdlib_arg.tvm /usr/bin/


