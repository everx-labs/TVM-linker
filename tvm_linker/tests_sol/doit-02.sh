address1=88a40cc7f39434e14bbb45909bc28f6ee36900d9ccb369c427ec7c799a685fac
msginit1=88a40cc7-msg-init.boc
msgbody1=88a40cc7-msg-body.boc
address2=d31fb5167484526d91c483f4d5463bcedb0535a003a08b0eaf92e95b503da93c
msginit2=d31fb516-msg-init.boc

rm -f *.tvc *.boc *.tmp

source set_env.sh

$linker --lib ../stdlib_sol.tvm ./contract02-a.code --abi-json ./contract02-a.abi.json
$linker --lib ../stdlib_sol.tvm ./contract02-b.code --abi-json ./contract02-b.abi.json

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
