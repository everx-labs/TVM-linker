import re
import os

def getFunctions():
	global functions
	for l in lines:
		match = re.search(r"Function (\S+)_external\s+: id=([0-9A-F]+), .*", l);
		if match:
			functions[match.group(1)] = match.group(2)
			# print match.group(1), match.group(2) 
	
def getExitCode():
	for l in lines:
		# print l
		match = re.match(r"TVM terminated with exit code (\d+)", l);
		if match:
			return int(match.group(1))
	assert False
	return -1
	
def getContractAddress():
	for l in lines:
		# print l
		match = re.match(r"Saved contract to file (.*)\.tvc", l);
		if match:
			return match.group(1)
	assert False
	return -1
	
def getStack():
	stack = []
	b = False
	for l in lines:
		if l == "--- Post-execution stack state ---------": 
			b = True
		elif l == "----------------------------------------":
			b = False
		elif b:
			ll = l.replace("Reference to ", "")
			stack.append(ll)
	return " ".join(stack)
		
def cleanup():
	if CONTRACT_ADDRESS:
		os.remove(CONTRACT_ADDRESS + ".tvc")

CONTRACT_ADDRESS = None

def compile(test_name):
	global lines, functions, CONTRACT_ADDRESS
	cleanup()
	print "Compiling " + test_name + "..."
	ec = os.system("./target/debug/tvm_linker --lib stdlib_sol.tvm ./tests/{} --debug >qqq".format(test_name))
	assert ec == 0, ec

	lines = [l.rstrip() for l in open("qqq").readlines()]

	functions = dict()
	getFunctions()
	CONTRACT_ADDRESS = getContractAddress()

SIGN = None

def exec_and_parse(method, params, expected_ec):
	global lines, SIGN
	sign = ("--sign " + SIGN) if SIGN else "";
	cmd = "./target/debug/tvm_linker {} test --body 00{}{} {} >qqqq".format(CONTRACT_ADDRESS, functions[method], params, sign)
	ec = os.system(cmd)
	assert ec == 0, ec

	lines = [l.rstrip() for l in open("qqqq").readlines()]

	ec = getExitCode()
	assert ec == expected_ec, "ec = {}".format(ec)
	
def expect_failure(method, params, expected_ec):
	exec_and_parse(method, params, expected_ec)
	print "  {} {} {}".format(method, params, expected_ec)
	
def expect_success(method, params, expected):
	exec_and_parse(method, params, 0)
	stack = getStack()
	if stack != expected:
		print "  {} {}".format(method, params)
		print "EXP: ", expected
		print "GOT: ", stack
		quit(1)
	print "  {} {} {}".format(method, params, expected)

compile('test_factorial.code')
expect_success('constructor', "", "")
expect_success('main', "0003", "6")
expect_success('main', "0006", "726")

compile('test_signature.code')
expect_failure('constructor', "", 100)
SIGN = "key1"
expect_success('constructor', "", "")
expect_success('get_role', "", "1")
SIGN = None
expect_failure('get_role', "", 100)
expect_failure('set_role', "", 9)
expect_failure('set_role', "01", 100)
SIGN = "key2"
expect_success('get_role', "", "0")
expect_success('set_role', "02", "")
expect_success('get_role', "", "2")

cleanup()
