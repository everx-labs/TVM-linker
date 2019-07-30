address1=c8b7d32afd3daa3429a6bb562cafd89668fa4d271bb7638cb6f2949062f6d76b
msginit1=c8b7d32a-msg-init.boc
msgbody1=c8b7d32a-msg-body.boc
address2=98cac5f04312dbaaa452b79ebc1c6868fb6e32e27e47cad6a2a4f260fe9753c7
msginit2=98cac5f0-msg-init.boc

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

$linker message --init -w 0 $address1
$linker message --init -w 0 $address2

$linker message -w 0 --abi-json contract05-a.abi.json --abi-method method_external \
	--abi-params "{\"anotherContract\":\"0x${address2}\", \"x\":\"257\"}" $address1

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
