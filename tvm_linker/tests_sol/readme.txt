1) Build sdk-emulator
cd /path/to/sdk-emulator
cargo clean
cargo build --release -j8 --bin ton_node_local
cargo build --release -j8 --bin create-msg

2) Build Telegram-Lite-Client from branch for-localnode
cd /path/to/Telegram-Lite-Client
mkdir build
cd build
cmake -G "Ninja" ..
ninja

3) copy test_suite.sample.json to test_suite.json and setup paths

4) copy config/log_cfg.yml to root of sdk-emulator
cp ../../../sdk-emulator/config/log_cfg.yml ../../../sdk-emulator/

5) Update stdlib_sol.tvm
cp ../../../sol2tvm/tests2/stdlib_sol.tvm ../stdlib_sol.tvm

6) run compile_all.sh to regenerate contracts

7) For run all tests
python3 test_suite3.py
For run for example 6th test
python3 test_suite3.py SoliditySuite.test_06
