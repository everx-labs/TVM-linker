address1=c8f0a9dffab49c3d892e46d6046e13d260d013fd0991895fa9980f9eabd26e77
msginit1=c8f0a9df-msg-init.boc
msgbody1=c8f0a9df-msg-body.boc
address2=f848373fe2f18c46c852e72f4287f442754bf680088add4674ce03549ad6460d
msginit2=f848373f-msg-init.boc

rm -f *.tvc *.boc *.tmp

source set_env.sh

$linker compile --lib ../stdlib_sol.tvm ./contract02-a.code --abi-json ./contract02-a.abi.json
$linker compile --lib ../stdlib_sol.tvm ./contract02-b.code --abi-json ./contract02-b.abi.json

if [ ! -f "${address1}.tvc" ]; then
  echo "FILE NOT FOUND! ${address1}.tvc"
  exit 1
fi

if [ ! -f "${address2}.tvc" ]; then
  echo "FILE NOT FOUND! ${address2}.tvc"
  exit 1
fi



$linker message --init -w 0 "${address1}"
$linker message --init -w 0 "${address2}"

$linker message -w 0 --abi-json contract02-a.abi.json --abi-method method_external \
	--abi-params "{\"anotherContract\":\"0x${address2}\"}" $address1

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
