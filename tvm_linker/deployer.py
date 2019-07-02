import os
from subprocess import Popen, PIPE
from time import sleep
import sys

LINKER_MESSAGE_EXT = '.code'

def compile_code(input_file):
	COMPILER = os.environ['SOL_COMPILER']
	COMPILER_CMD = COMPILER + ' ' + input_file + ' --tvm'
	os.system(COMPILER_CMD)
	linker_file = input_file.split('.')[0] + LINKER_MESSAGE_EXT
	LINKER = os.environ['TVM_LINKER']
	LINKER_LIB = os.environ['TVM_LINKER_LIB']
	LINKER_CMD = LINKER + ' --lib ' +  LINKER_LIB + ' ' + linker_file + ' --debug'
	p = Popen(LINKER_CMD.split(' '), stdout=PIPE)
	text = p.stdout.read()
	#get address - last word of output is name of file, starting with address
	return text.rsplit(' ', 1)[1].split('.')[0]

def create_msg(dst_addr):
	CREATE_MSG = '../../sdk-emulator/target/debug/create-msg'
	CREATE_MSG_VALUE = '1000000000'
	CREATE_MSG = os.environ['CREATE_MSG']
	CREATE_MSG_OUT = 'sendmoney.boc'
	CREATE_MSG_CMD = CREATE_MSG + ' --type transfer --src 0000000000000000000000000000000000000000000000000000000000000000 --dst ' + dst_addr + ' --value ' + CREATE_MSG_VALUE + ' --out ' + CREATE_MSG_OUT
	os.system(CREATE_MSG_CMD)
	return CREATE_MSG_OUT

def init_msg(msg_addr):
	LINKER = os.environ['TVM_LINKER']
	INIT_MSG_CMD = LINKER + ' ' + msg_addr + ' message --init'
	p = Popen(INIT_MSG_CMD.split(' '), stdout=PIPE)
	text = p.stdout.read()
	#get name of file - get last word and remove everything after
	return text.rsplit(' ', 1)[1].split('.')[0] + '.boc'

if __name__ == '__main__':
	input_file = sys.argv[1]
	addr = compile_code(input_file)
	print('resulting addr: ' + addr)
	create_msg_file = create_msg(addr)
	init_msg_file = init_msg(addr)
	print('prepared ' + init_msg_file)
	LITE_CLIENT = os.environ['LITE_CLIENT']
	LITE_CLIENT_CONFIG = os.environ['LITE_CLIENT_CONFIG']
	p = Popen([LITE_CLIENT, '-C', LITE_CLIENT_CONFIG, '-f', create_msg_file])
	sleep(5)
	p = Popen([LITE_CLIENT, '-C', LITE_CLIENT_CONFIG, '-f', init_msg_file])
	sleep(5)
	p = Popen([LITE_CLIENT, '-C', LITE_CLIENT_CONFIG, '-a', addr])
	#sleep(2)
	#p.stdin.write('getaccount ' + '230aa5cd9e512c5d481b6da4842476675553439d7897ffb7f794ec192564f934' + '\n')