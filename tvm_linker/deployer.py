import os
from subprocess import Popen, PIPE
from time import sleep

LINKER = 'target/debug/tvm_linker'
LINKER_LIB = '/media/sf_UbuntuShared/Projects/TVM-linker/tvm_linker/stdlib_sol.tvm'
LINKER_MESSAGE_EXT = '.code'
LINKER_MESSAGE = COMPILER_INPUT_FILE_PATH + COMPILER_INPUT_FILE_NAME + LINKER_MESSAGE_EXT
LINKER_CMD = LINKER + ' --lib ' +  LINKER_LIB + ' ' + LINKER_MESSAGE + ' --debug'
LINKER_CMD_NO_LIB = LINKER + ' ' + LINKER_MESSAGE + ' --debug'

LITE_CLIENT = '../../Telegram-Lite-Client-May24/build/test-lite-client'
LITE_CLIENT_CONFIG_LOCAL = '/media/sf_UbuntuShared/Projects/Telegram-Lite-Client-May24/ton-labs-local-node.config.json'
LITE_CLIENT_CONFIG = '/media/sf_UbuntuShared/Projects/Telegram-Lite-Client-May24/ton-labs-net.config.json'
LITE_CLIENT_CMD = '-C ' + LITE_CLIENT_CONFIG

def compile_code(input_file):
	COMPILER_CMD = COMPILER + ' ' + input_file + ' --tvm'
	p = Popen(LINKER_CMD.split(' '), stdout=PIPE)
	text = p.stdout.read()
	#get address - last word of output is name of file, starting with address
	return text.rsplit(' ', 1)[1].split('.')[0]

def create_msg(dst_addr):
	CREATE_MSG = '../../sdk-emulator/target/debug/create-msg'
	CREATE_MSG_VALUE = '1000000000'
	CREATE_MSG_OUT = 'sendmoney_save.boc'
	CREATE_MSG_CMD = CREATE_MSG + ' --type transfer --src 0000000000000000000000000000000000000000000000000000000000000000 --dst ' + dst_addr + ' --value ' + CREATE_MSG_VALUE + ' --out ' + CREATE_MSG_OUT
	os.system(CREATE_MSG_CMD)

def init_msg(msg_addr):
	INIT_MSG_CMD = LINKER + ' ' + msg_addr + ' message --init'
	p = Popen(INIT_MSG_CMD.split(' '), stdout=PIPE)
	text = p.stdout.read()
	#get name of file - get last word and remove everything after
	return text.rsplit(' ', 1)[1].split('.')[0] + '.boc'

if __name__ == '__main__':
	addr = compile_code(COMPILER_INPUT_FILE)
	print('resulting addr: ' + addr)
	create_msg(addr)
	init_msg_file = init_msg(addr)
	print('prepared ' + init_msg_file)
	p = Popen([LITE_CLIENT, '-C', LITE_CLIENT_CONFIG_LOCAL, '-f', CREATE_MSG_OUT])
	sleep(5)
	p = Popen([LITE_CLIENT, '-C', LITE_CLIENT_CONFIG_LOCAL, '-f', init_msg_file])
	sleep(5)
	p = Popen([LITE_CLIENT, '-C', LITE_CLIENT_CONFIG_LOCAL, '-a', addr])
	#sleep(2)
	#p.stdin.write('getaccount ' + '230aa5cd9e512c5d481b6da4842476675553439d7897ffb7f794ec192564f934' + '\n')