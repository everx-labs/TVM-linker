address1=d178527ff8863b91dcf5011de00da4b6115a0e05bcbfe7e9ec793fcde1d799d2
msginit1=d178527f-msg-init.boc
msgbody1=d178527f-msg-body.boc
address2=7963c33332e71bcfacd18612676fd729f51546167e43cc5bfea233fba0d208a0
msginit2=7963c333-msg-init.boc

rm -f *.tvc *.boc *.tmp

source set_env.sh

$linker compile --lib ../stdlib_sol.tvm ./contract03-a.code --abi-json ./contract03-a.abi.json
$linker compile --lib ../stdlib_sol.tvm ./contract03-b.code --abi-json ./contract03-b.abi.json

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

$linker message -w 0 --abi-json contract03-a.abi.json --abi-method method_external \
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
