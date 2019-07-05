pkill ton-node
sdk_dir=../../../tmp/sdk-emulator/
cd ${sdk_dir}
rm ./log/output.log
#rm -rf ./shard_0000000000000000
#rm -rf ./shardes
rm -rf ./workchains
# ./target/debug/ton-node test >/dev/null &
./target/debug/ton-node --config dumy --localhost  >/dev/null &

