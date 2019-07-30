address=5780578dc30182396e0eca67ecd8db83a64b72819dc8093afc328c3865b9cbfb
address1short=`echo $address | cut -c 1-8`
msginit=$address1short-msg-init.boc
msgbody=$address1short-msg-body.boc

rm -f *.tvc *.boc *.tmp

set +x

source set_env.sh

$linker compile --lib ../stdlib_sol.tvm ./contract01.code --abi-json ./contract01.abi.json

if [ ! -f "${address}.tvc" ]; then
  echo "FILE NOT FOUND! ${address}.tvc"
  exit 1
fi


$linker message $address --init -w 0
echo    $linker message $address -w 0 --abi-json contract01.abi.json --abi-method main_external --abi-params "{\"a\":\"0x1234\"}"
$linker message $address -w 0 --abi-json contract01.abi.json --abi-method main_external --abi-params "{\"a\":\"0x1234\"}"

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
