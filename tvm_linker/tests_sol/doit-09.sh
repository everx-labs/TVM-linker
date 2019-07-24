address1=7b13b56196d37a164e7ba9d9544626bfe7a02854dc2b9a4d5edea798c8ba7b7f
address1short=`echo $address1 | cut -c 1-8`
msginit1=$address1short-msg-init.boc
msgbody1=$address1short-msg-body.boc
address2=1d1fc2e851a5a19cad417a08b94ba78af91279d53113012bccfb8e0140e73532
address2short=`echo $address2 | cut -c 1-8`
msginit2=$address2short-msg-init.boc

rm -f *.tvc *.boc *.tmp

source set_env.sh

$linker compile --lib ../stdlib_sol.tvm ./contract09-a.code --abi-json ./contract09-a.abi.json
$linker compile --lib ../stdlib_sol.tvm ./contract09-b.code --abi-json ./contract09-b.abi.json

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

$linker message $address1 -w 0 --abi-json contract09-a.abi.json --abi-method sendMoneyAndNumber_external \
	--abi-params "{\"remote\":\"0x${address2}\", \"number\":\"257\"}"

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
