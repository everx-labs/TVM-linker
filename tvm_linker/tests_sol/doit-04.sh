address1=cfae0a88cda4e38732443b9156144b3bb92d2c8a49932156bd4324012eb42851
msginit1=cfae0a88-msg-init.boc
msgbody1=cfae0a88-msg-body.boc
address2=32203977eee9900af0167ea87c879d70438098f9d546de9f5a3e5c6cfd281247
msginit2=32203977-msg-init.boc

rm -f *.tvc *.boc *.tmp

source set_env.sh

$linker compile --lib ../stdlib_sol.tvm ./contract04-a.code --abi-json ./contract04-a.abi.json
$linker compile --lib ../stdlib_sol.tvm ./contract04-b.code --abi-json ./contract04-b.abi.json

if [ ! -f "${address1}.tvc" ]; then
  echo "FILE NOT FOUND! ${address1}.tvc"
  exit 1
fi

if [ ! -f "${address2}.tvc" ]; then
  echo "FILE NOT FOUND! ${address2}.tvc"
  exit 1
fi

$linker message --init -w 0 $address1
$linker message --init -w 0 $address2

$linker message -w 0 --abi-json contract04-a.abi.json --abi-method method_external \
	--abi-params "{\"anotherContract\":\"0x${address2}\", \"amount\":\"5000000\"}" $address1

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
