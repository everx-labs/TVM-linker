FROM eclipse/ubuntu_jdk8:latest as build-tvm-linker
LABEL stage=intermediate-tvm-linker
USER root
RUN echo deb http://ubuntu-cloud.archive.canonical.com/ubuntu precise-updates/grizzly main >>/etc/apt/sources.list
RUN apt-get update
RUN apt-get install cargo -y

WORKDIR /home/user
COPY . TVM-linker
WORKDIR /home/user/TVM-linker/tvm_linker
RUN cargo build --release;

RUN chmod a+x stdlib_c.tvm
RUN chmod a+x stdlib_sol.tvm
RUN chmod a+x stdlib_arg.tvm
RUN chmod a+x target/release/tvm_linker


FROM alpine
COPY --from=build-ton-compiler /home/user/TVM-linker/tvm_linker/target/release/tvm_linker /usr/bin/
COPY --from=build-ton-compiler /home/user/TVM-linker/tvm_linker/stdlib_c.tvm /usr/bin/
COPY --from=build-ton-compiler /home/user/TVM-linker/tvm_linker/stdlib_sol.tvm /usr/bin/
COPY --from=build-ton-compiler /home/user/TVM-linker/tvm_linker/stdlib_arg.tvm /usr/bin/


