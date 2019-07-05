address1=9a7dc77d7a5c38fc2274570766855fc4d4f6c6267e76990cf73c202bb4dfefb6
msginit1=9a7dc77d-msg-init.boc
msgbody1=9a7dc77d-msg-body.boc
address2=3cfd3ea7dd58ad1ef43d2e217773cf347f36719144ba6e6895775b70506a4eb3
msginit2=3cfd3ea7-msg-init.boc

rm -f *.tvc *.boc *.tmp

source set_env.sh

$linker --lib ../stdlib_sol.tvm ./contract02-a.code --abi-json ./contract02-a.abi.json
$linker --lib ../stdlib_sol.tvm ./contract02-b.code --abi-json ./contract02-b.abi.json

$linker $address1 message --init -w 0
$linker $address2 message --init -w 0

$linker $address1 message -w 0 --abi-json contract02-a.abi.json --abi-method method_external \
	--abi-params "{\"anotherContract\":\"0x${address2}\"}"

zeroes=0000000000000000000000000000000000000000000000000000000000000000

$emulator/create-msg --type transfer --src $zeroes --dst $address1 --value 1000000 --out sendmoney1.boc
$emulator/create-msg --type transfer --src $zeroes --dst $address2 --value 1000000 --out sendmoney2.boc


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
$tlc -a 0:$address1
sleep 5
echo "-------------------------------"
$tlc -a 0:$address2
