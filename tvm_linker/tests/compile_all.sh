set -e

if [ "$SOLC_PATH" = "" ]; then
	SOLC_PATH=../../../sol2tvm/build/solc
fi

for filename in *.sol; do
    basename="$(echo $filename | sed 's/\.sol//')"
    echo "Compiling $filename..."
    ${SOLC_PATH}/solc $filename --tvm >"$basename".code
    ${SOLC_PATH}/solc $filename --tvm_abi >"$basename".abi.json
done