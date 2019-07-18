address1=03d796589e3f02cf70c707743dbeb0074edb1e583ba245448d0f75fc846130be
msginit1=03d79658-msg-init.boc
msgbody1=03d79658-msg-body.boc
address2=0f2dccd895dfa554a407381dbcc61dff27e1c4a92172f984eb9f127df61c2780
msginit2=0f2dccd8-msg-init.boc

rm -f *.tvc *.boc *.tmp

source set_env.sh

$linker compile --lib ../stdlib_sol.tvm ./contract05-a.code --abi-json ./contract05-a.abi.json
$linker compile --lib ../stdlib_sol.tvm ./contract05-b.code --abi-json ./contract05-b.abi.json

if [ ! -f "${address1}.tvc" ]; then
  echo "FILE NOT FOUND! ${address1}.tvc"
  exit 1
fi

if [ ! -f "${address2}.tvc" ]; then
  echo "FILE NOT FOUND! ${address2}.tvc"
  exit 1
fi

$linker message $address1 --init -w 0
$linker message $address2 --init -w 0

$linker message $address1 -w 0 --abi-json contract05-a.abi.json --abi-method method_external \
	--abi-params "{\"anotherContract\":\"0x${address2}\", \"x\":\"257\"}"

zeroes=0000000000000000000000000000000000000000000000000000000000000000

$emulator/create-msg --type transfer --src $zeroes --dst $address1 --value 10000000 --out sendmoney1.boc
$emulator/create-msg --type transfer --src $zeroes --dst $address2 --value 10000000 --out sendmoney2.boc


echo "-------------------------------"
$tlc -f sendmoney1.boc
$tlc -f sendmoney2.boc
echo "-------------------------------"
sleep 5
echo "-------------------------------"
$tlc -a 0:$address1
$tlc -a 0:$address2

echo "-------------------------------"
$tlc -f $msginit1
$tlc -f $msginit2
echo "-------------------------------"
sleep 5

echo "-------------------------------"
$tlc -a 0:$address1
$tlc -a 0:$address2

echo "-------------------------------"
$tlc -f $msgbody1
echo "-------------------------------"
sleep 5
echo "-------------------------------"
$tlc -a 0:$address2
sleep 5
echo "-------------------------------"
$tlc -a 0:$address1
