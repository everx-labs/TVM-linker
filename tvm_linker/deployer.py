import os
from subprocess import Popen, PIPE
from time import sleep

LINKER = '/media/sf_UbuntuShared/Projects/TVM-linker/tvm_linker/target/debug/tvm_linker'
LINKER_LIB = '/media/sf_UbuntuShared/Projects/TVM-linker/tvm_linker/stdlib_sol.tvm'
LINKER_MESSAGE = '/media/sf_UbuntuShared/Projects/TVM-linker/tvm_linker/tests/savemsg.code'
LINKER_CMD = LINKER + ' --lib ' +  LINKER_LIB + ' ' + LINKER_MESSAGE + ' --debug'
LINKER_CMD_NO_LIB = LINKER + ' ' + LINKER_MESSAGE + ' --debug'

CREATE_MSG = '/media/sf_UbuntuShared/Projects/sdk-emulator/target/debug/create-msg'
CREATE_MSG_DST_ADDR = '230aa5cd9e512c5d481b6da4842476675553439d7897ffb7f794ec192564f934'
CREATE_MSG_VALUE = '1000000000'
CREATE_MSG_OUT = 'sendmoney_save.boc'
CREATE_MSG_CMD = CREATE_MSG + ' --type transfer --src 0000000000000000000000000000000000000000000000000000000000000000 --dst ' + CREATE_MSG_DST_ADDR + ' --value ' + CREATE_MSG_VALUE + ' --out ' + CREATE_MSG_OUT

INIT_MSG_ADDR = '230aa5cd9e512c5d481b6da4842476675553439d7897ffb7f794ec192564f934'
INIT_MSG_CMD = LINKER + ' ' + INIT_MSG_ADDR + ' message --init'
INIT_MSG_OUT = '230aa5cd-msg-init.boc'

LITE_CLIENT = '/home/simon/Projects/Telegram-Lite-Client-May24/build/test-lite-client'
LITE_CLIENT_CONFIG_LOCAL = '/media/sf_UbuntuShared/Projects/Telegram-Lite-Client-May24/ton-labs-local-node.config.json'
LITE_CLIENT_CONFIG = '/media/sf_UbuntuShared/Projects/Telegram-Lite-Client-May24/ton-labs-net.config.json'
LITE_CLIENT_CMD = '-C ' + LITE_CLIENT_CONFIG

if __name__ == '__main__':
	os.system(LINKER_CMD_NO_LIB)
	os.system(CREATE_MSG_CMD)
	os.system(INIT_MSG_CMD)
	p = Popen([LITE_CLIENT, '-C', LITE_CLIENT_CONFIG_LOCAL, '-f', CREATE_MSG_OUT, '-f', INIT_MSG_OUT, '-a', INIT_MSG_ADDR], stdin=PIPE, close_fds=True)
	#sleep(2)
	#p.stdin.write('getaccount ' + '230aa5cd9e512c5d481b6da4842476675553439d7897ffb7f794ec192564f934' + '\n')