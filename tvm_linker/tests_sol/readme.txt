How to run tests?

0) build TVM-linker
cd ..
cargo clean
cargo build --release -j8 --bin tvm_linker

1) Build sdk-emulator
cd ../../../sdk-emulator
cargo clean
cargo build --release -j8 --bin ton_node_local
cargo build --release -j8 --bin create-msg

2) copy config/log_cfg.yml to root of sdk-emulator
cp ../../../sdk-emulator/config/log_cfg.yml ../../../sdk-emulator/
See logs in ../../../sdk-emulator/log

3) Build Telegram-Lite-Client from branch for-localnode
cd ../../../Telegram-Lite-Client
git checkout for-localnode
mkdir build
cd build
cmake -G "Ninja" ..
ninja

4) copy test_suite.sample.json to test_suite.json and setup paths

5) Update stdlib_sol.tvm
cp ../../../sol2tvm/tests2/stdlib_sol.tvm ../stdlib_sol.tvm

6) recompile contracts
SOLC_PATH=../../../sol2tvm/build/solc/ bash ./compile_all.sh

7) For run all tests
python3 test_suite3.py
For run for example 6th test
python3 test_suite3.py SoliditySuite.test_06
