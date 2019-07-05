import re, os, time

def getContractAddress():
	for l in lines:
		# print l
		match = re.match(r"Saved contract to file (.*)\.tvc", l);
		if match:
			return match.group(1)
	assert False
	return -1

CONTRACT_ADDRESS = None

def compile_ex(source_file, lib_file):
	global lines, functions, CONTRACT_ADDRESS
	print("Compiling " + source_file + "...")
	lib = "--lib ../" + lib_file if lib_file else ""
	ec = os.system("../target/debug/tvm_linker {} ./{} --debug > compile_log.tmp".format(lib, source_file))
	if ec != 0:
		error("COMPILATION FAILED!")
	lines = [l.rstrip() for l in open("compile_log.tmp").readlines()]
	CONTRACT_ADDRESS = getContractAddress()

def error(msg):
	print "ERROR!", msg
	quit(1)
	
def tlc(cmd):
	os.system("../../../lite-client-build/test-lite-client -C ../../../local-node-config.json " + cmd)

def sendmoney():
	print "Sending grams to", CONTRACT_ADDRESS
	zeroes = "0000000000000000000000000000000000000000000000000000000000000000"
	cmd = "../../../sdk-emulator/target/debug/create-msg --type transfer --src {} --dst {} --value 1000000 --out sendmoney.boc"
	os.system(cmd.format(zeroes, CONTRACT_ADDRESS));
	tlc("-f sendmoney.boc");
	time.sleep(3);
	tlc("-a 0:{} | tee sendmoney_log.tmp".format(CONTRACT_ADDRESS));

compile_ex('contract01.code', 'stdlib_sol.tvm')
print CONTRACT_ADDRESS
sendmoney()
print CONTRACT_ADDRESS[0:8]
