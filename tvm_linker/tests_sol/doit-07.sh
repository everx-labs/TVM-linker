address1=e81f5144ff7888b42b969ee8b3e95d2f1db1f0905c65753ee5d3893e10d8b4d6
msginit1=e81f5144-msg-init.boc
msgbody1=e81f5144-msg-body.boc
address2=4724a7afea47104d62d22dd00acf7b212d2c04dfd6154b709ea422715034242c
msginit2=4724a7af-msg-init.boc

rm -f *.tvc *.boc *.tmp

source set_env.sh

$linker --lib ../stdlib_sol.tvm ./contract06-a.code --abi-json ./contract06-a.abi.json
$linker --lib ../stdlib_sol.tvm ./contract06-b.code --abi-json ./contract06-b.abi.json

if [ ! -f "${address1}.tvc" ]; then
  echo "FILE NOT FOUND! ${address1}.tvc"
  exit 1
fi

if [ ! -f "${address2}.tvc" ]; then
  echo "FILE NOT FOUND! ${address2}.tvc"
  exit 1
fi

$linker $address1 message --init -w 0
$linker $address2 message --init -w 0

$linker $address1 message -w 0 --abi-json contract06-a.abi.json --abi-method method_external \
	--abi-params "{\"anotherContract\":\"0x${address2}\"}"

zeroes=0000000000000000000000000000000000000000000000000000000000000000

$emulator/create-msg --type transfer --src $zeroes --dst $address1 --value 5000000 --out sendmoney1.boc
$emulator/create-msg --type transfer --src $zeroes --dst $address2 --value 5000000 --out sendmoney2.boc


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
