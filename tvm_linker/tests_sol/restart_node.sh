pkill ton-node
source set_env.sh
cd ${sdk_dir}
rm ./log/output.log
rm -rf ./workchains
./target/release/ton-node --config dumy --localhost  >/dev/null &

