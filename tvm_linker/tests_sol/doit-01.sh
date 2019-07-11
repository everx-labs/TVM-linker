address=aceb641ba8fb487bd2bec7e94f0532496528cafbe8b84ff229d83106e247fc59
msginit=aceb641b-msg-init.boc
msgbody=aceb641b-msg-body.boc

rm -f *.tvc *.boc *.tmp

source set_env.sh

$linker --lib ../stdlib_sol.tvm ./contract01.code --abi-json ./contract01.abi.json

if [ ! -f "${address}.tvc" ]; then
  echo "FILE NOT FOUND! ${address}.tvc"
  exit 1
fi


$linker $address message --init -w 0
$linker $address message -w 0 --abi-json contract01.abi.json --abi-method main_external --abi-params "{\"a\":\"0x1234\"}"

zeroes=0000000000000000000000000000000000000000000000000000000000000000

$emulator/create-msg --type transfer --src $zeroes --dst $address --value 1000000 --out sendmoney.boc

echo "-------------------------------"
$tlc -f sendmoney.boc
echo "-------------------------------"
sleep 5
echo "-------------------------------"
$tlc -a 0:$address
echo "-------------------------------"
$tlc -f $msginit
echo "-------------------------------"
sleep 5
echo "-------------------------------"

$tlc -a 0:$address
echo "-------------------------------"
$tlc -f $msgbody
echo "-------------------------------"
sleep 5
echo "-------------------------------"
$tlc -a 0:$address
