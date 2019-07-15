address1=c95fd10e9d5f3e513f29c5bdbb64ee6230392fdcaf534b66897859ca6f2c20b3
msginit1=c95fd10e-msg-init.boc
msgbody1=c95fd10e-msg-body.boc
address2=989825df618c84b1d7ae1d15e01c35d85ed7f1f39dd77f58f73c3fab6190a92f
msginit2=989825df-msg-init.boc

rm -f *.tvc *.boc *.tmp

source set_env.sh

$linker --lib ../stdlib_sol.tvm ./contract04-a.code --abi-json ./contract04-a.abi.json
$linker --lib ../stdlib_sol.tvm ./contract04-b.code --abi-json ./contract04-b.abi.json

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

$linker $address1 message -w 0 --abi-json contract04-a.abi.json --abi-method method_external \
	--abi-params "{\"anotherContract\":\"0x${address2}\", \"amount\":\"5000000\"}"

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
