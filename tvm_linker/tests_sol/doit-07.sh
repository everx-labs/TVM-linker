address1=4c559656f23046e36b5743f022fc5760a8d99fddc10875fdbab7ab9814f71a95
msginit1=4c559656-msg-init.boc
msgbody1=4c559656-msg-body.boc
address2=17ed3e4f2d7aceaacadeb88bef73e17645e85568451488c275d5df6dc21a00bc
msginit2=17ed3e4f-msg-init.boc

rm -f *.tvc *.boc *.tmp

source set_env.sh

$linker compile --lib ../stdlib_sol.tvm ./contract06-a.code --abi-json ./contract06-a.abi.json
$linker compile --lib ../stdlib_sol.tvm ./contract06-b.code --abi-json ./contract06-b.abi.json

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

$linker message -w 0 --abi-json contract07-a.abi.json --abi-method method_external \
	--abi-params "{\"anotherContract\":\"0x${address2}\"}" $address1

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
