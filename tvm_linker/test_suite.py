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
		

def compile(test_name):
	global lines, functions, CONTRACT_ADDRESS
	print "Compiling " + test_name + "..."
	ec = os.system("./target/debug/tvm_linker --lib stdlib_sol.tvm ./tests/{} --debug >qqq".format(test_name))
	assert ec == 0, ec

	lines = [l.rstrip() for l in open("qqq").readlines()]

	functions = dict()
	getFunctions()
	CONTRACT_ADDRESS = "2ed48789a1b2912dfbd28f40df5cdf5c5e753b220749cfa455d698acf1c4d10b"

def expect_success(method, params, expected):
	global lines
	cmd = "./target/debug/tvm_linker {} test --body 00{}{} >qqqq".format(CONTRACT_ADDRESS, functions[method], params)
	ec = os.system(cmd)
	assert ec == 0, ec

	lines = [l.rstrip() for l in open("qqqq").readlines()]

	ec = getExitCode()
	assert ec == 0, "ec = " + ec
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
