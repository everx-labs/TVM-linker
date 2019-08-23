import re
import os

TVM_PATH = './target/release/tvm_linker'
# TVM_PATH = './target/debug/tvm_linker'

def getFunctions():
	global functions
	for l in lines:
		match = re.search(r"Function (\S+)_external\s+: id=([0-9A-F]+)", l);
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

def compile1(source_file, lib_file):
	cleanup()
	global lines, functions, CONTRACT_ADDRESS
	print("Compiling " + source_file + "...")
	lib = "--lib " + lib_file if lib_file else ""
	cmd = "{} compile ./tests/{} {} --debug > compile_log.tmp".format(TVM_PATH, source_file, lib)
	# print cmd
	ec = os.system(cmd)
	if ec != 0:
		error("COMPILATION FAILED!")

	lines = [l.rstrip() for l in open("compile_log.tmp").readlines()]
	# os.remove("compile_log.tmp")

	functions = dict()
	getFunctions()
	CONTRACT_ADDRESS = getContractAddress()
	# print functions, CONTRACT_ADDRESS

def compile2(source_name, directory = "tests_sol"):
	cleanup()
	global lines, functions, CONTRACT_ADDRESS
	print("Compiling " + source_name + "...")
	lib_file = "stdlib_sol.tvm"
	source_file = "./" + directory + "/{}.code".format(source_name)
	abi_file = "./" + directory + "/{}.abi.json".format(source_name)
	
	cmd = "{} compile {} --abi-json {} --lib {} --debug > compile_log.tmp"
	cmd = cmd.format(TVM_PATH, source_file, abi_file, lib_file)
	# print cmd
	ec = os.system(cmd)
	if ec != 0:
		error("COMPILATION FAILED!")

	lines = [l.rstrip() for l in open("compile_log.tmp").readlines()]
	# os.remove("compile_log.tmp")

	functions = dict()
	getFunctions()
	CONTRACT_ADDRESS = getContractAddress()
	# print functions, CONTRACT_ADDRESS

SIGN = None

def error(msg):
	print "ERROR!", msg
	quit(1)

def exec_and_parse(cmd, expected_ec):
	global lines
	# print cmd
	ec = os.system(cmd)
	assert ec == 0, ec

	lines = [l.rstrip() for l in open("exec_log.tmp").readlines()]
	# os.remove("exec_log.tmp")

	ec = getExitCode()
	assert ec == expected_ec, "ec = {}".format(ec)

def build_cmd_exec_and_parse(method, params, expected_ec, options):
	global SIGN
	if "--trace" not in options:
		options = options + " --trace"
	sign = ("--sign " + SIGN) if SIGN else "";
	if method and method not in functions:
		error("Cannot find method '{}'".format(method)) 
	if method == None:
		body = ""
	elif method == "":
		body = "--body 00"
	else:
		id = functions[method]
		body = "--body 00{}{}".format(id, params)
	cmd = "{} test {} {} {} {} >exec_log.tmp".format(TVM_PATH, CONTRACT_ADDRESS, body, sign, options)
	exec_and_parse(cmd, expected_ec)
	
def build_cmd_exec_and_parse2(abi_json_file, abi_method, abi_params, tvm_linker_options, expected_ec):
	if "--trace" not in tvm_linker_options:
		tvm_linker_options = tvm_linker_options + " --trace"
	cmd = "{} \
test {} \
--abi-json {} \
--abi-method {} \
--abi-params '{}' \
{} \
>exec_log.tmp"\
.format(TVM_PATH, CONTRACT_ADDRESS, "./tests/" + abi_json_file + ".abi.json", abi_method, abi_params, tvm_linker_options)
	exec_and_parse(cmd, expected_ec)

def expect_failure(method, params, expected_ec, options):
	build_cmd_exec_and_parse(method, params, expected_ec, options)
	print("OK:  {} {} {}".format(method, params, expected_ec))
	
def checkStack(method, params, expected_stack):
	stack = getStack()
	if expected_stack != None and stack != expected_stack:
		print("Failed:  {} {}".format(method, params))
		print("EXP: ", expected_stack)
		print("GOT: ", stack)
		quit(1)
	print("OK:  {} {} {}".format(method, params, expected_stack))

def expect_success(method, params, expected, options):
	build_cmd_exec_and_parse(method, params, 0, options)
	checkStack(method, params, expected)

def expect_success2(abi_json_file, abi_method, abi_params, expected_stack, tvm_linker_options):
	build_cmd_exec_and_parse2(abi_json_file, abi_method, abi_params, tvm_linker_options, 0)
	checkStack(abi_method, abi_params, expected_stack)

def expect_output(regex):
	for l in lines:
		match = re.search(regex, l);
		if match:
			print "> ", match.group(0)
			return
	assert False, regex

def testOld():
	global SIGN
	compile1('test_factorial.code', 'stdlib_sol.tvm')
	expect_success('constructor', "", "", "")
	expect_output(r"Gas used:.*")
	expect_success('main', "0003", "6", "")
	expect_success('main', "0006", "726", "")

	compile1('test_signature.code', 'stdlib_sol.tvm')
	expect_failure('constructor', "", 100, "")
	SIGN = "key1"
	expect_success('constructor', "", "", "--trace")
	expect_success('get_role', "", "1", "")
	SIGN = None
	expect_failure('get_role', "", 100, "")
	expect_failure('set_role', "", 9, "")
	expect_failure('set_role', "01", 100, "")
	SIGN = "key2"
	expect_success('get_role', "", "0", "")
	expect_success('set_role', "02", "", "")
	expect_success('get_role', "", "2", "")

	SIGN = None
	compile1('test_inbound_int_msg.tvm', None)
	expect_success("", "", "-1", "--internal 15000000000")

	compile1('test_send_int_msg.tvm', 'stdlib_sol.tvm')
	expect_success(None, "", None, "")	# check empty input (deploy)
	expect_success('main', "", None, "--internal 0 --decode-c6")
	expect_output(r"destination : 0:0+007f")
	expect_output(r"CurrencyCollection: Grams.*value = 1000]")

	compile1('test_send_ext_msg.tvm', 'stdlib_sol.tvm')
	expect_success(None, "", None, "")	# check empty input (deploy)
	expect_success('main', "", None, "--internal 0 --decode-c6")
	expect_output(r"destination : AddrNone")
	expect_output(r"body_hex: 00003039")

	compile1('test_send_int_msg.tvm', 'stdlib_sol.tvm')
	expect_success('main', "", None, "--decode-c6")
	expect_output(r"destination : 0:0+007f")
	expect_output(r"CurrencyCollection: Grams.*value = 1000]")
		
	compile1('test_send_msg.code', 'stdlib_sol.tvm')
	expect_success(None, "", None, "")	# check empty input (deploy)
	expect_success('get_allowance', "1122334455660000000000000000000000000000000000000000005544332211", None, "--internal 0 --decode-c6 --trace")
	expect_output(r"destination : 0:1122334455660000000000000000000000000000000000000000005544332211")
	expect_output(r"body_hex: 001a0b56870000000000000000")

		# '''
	compile1('test_msg_sender.code', None)
	expect_success(None, "", None, "--internal 0 --trace")	# check empty input (deploy)


	compile1('test_msg_sender2.code', 'stdlib_sol.tvm')
	# check internal message
	expect_success('main', "", "0", "--internal 0")
	# check external message
	expect_success('main', "", "0", "")

	#check msg.value
	compile1('test_msg_value.code', 'stdlib_sol.tvm')
	expect_success("main", "", "15000000000", "--internal 15000000000")


	#check msg.sender
	compile1('test_balance.code', 'stdlib_sol.tvm')
	expect_success("main", "", "100000000000", "--internal 0")

def testOld2():
	
	compile2('contract09-a')
	expect_success('sendMoneyAndNumber', ("12" * 32) + ("7" * 16), None, "--internal 0 --decode-c6")
	expect_output(r"destination : 0:12121212")
	expect_output(r"CurrencyCollection: Grams.*value = 3000000]")
	expect_output(r"body.*119, 119, 119, 119, 119, 119, 119, 119, 128\]")

	compile2('test20', 'tests')
	expect_success('test19', "0000007F000000FF", None, "--internal 0 --decode-c6")
	expect_output(r"body.*0, 0, 0, 127, 0, 0, 0, 255, 128\]")
	expect_success('test19', "1122334455667788", None, "--internal 0 --decode-c6")
	expect_output(r"body.*17, 34, 51, 68, 85, 102, 119, 136, 128\]")

	#check tvm_balance
	compile1('test_tvm_balance.code', 'stdlib_sol.tvm')
	expect_success("main", "", "100000000000", "--internal 0")

	# TODO: cannot predict value of now, need to test it somehow
	#check tvm_now
	compile1('test_now.code', 'stdlib_sol.tvm')
	# expect_success("main", "", "1564090968", "--internal 0")
	
	# TODO: cannot predict value of now, need to test it somehow
	#check now global variable
	compile1('test_now_variable.code', 'stdlib_sol.tvm')
	expect_success("main", "", "", "--internal 0")
	

	#check tvm_address
	compile1('test_tvm_address.code', 'stdlib_sol.tvm')
	expect_success("main", "", "0", "--internal 0")

	#check tvm_block_lt
	compile1('test_tvm_block_lt.code', 'stdlib_sol.tvm')
	expect_success("main", "", "0", "--internal 0")

	#check tvm_trans_lt
	compile1('test_tvm_trans_lt.code', 'stdlib_sol.tvm')
	expect_success("main", "", "0", "--internal 0")

	#check tvm_rand_seed
	compile1('test_tvm_rand_seed.code', 'stdlib_sol.tvm')
	expect_success("main", "", "0", "--internal 0")
	
	#check array length enlargement
	compile1('test_array_size.code', 'stdlib_sol.tvm')
	expect_success("main", "0006000C", "12", "--internal 0")

	#check array length shrink
	compile1('test_array_size.code', 'stdlib_sol.tvm')
	expect_success("main", "000C0006", "6", "--internal 0")

def testArrays():
	#it maybe '--sign key1' or '--internal 0' - test will work correctly
	linker_options = ""
	compile2('test_arrays', 'tests')
	ar1 = '1,'*500 + "1";
	ar2 = '2,'*500 + "2";
	expect_success2("test_arrays", "pair8_external", '{"arr1": [' + ar1 + '], "arr2": [' + ar2 + ']}', "3", linker_options)
	expect_success2("test_arrays", "pair64_external", '{"arr1": [1,2,3,4,5,6,7,8,9,10], "arr2": [1,2,3,4,5,6]}', "2", linker_options)
	expect_success2("test_arrays", "pair64_external", '{"arr1": [1,2,3,4,5,6,7,8,9,10], "arr2": [1,2,3,4,5,6]}', "2", linker_options)
	expect_success2("test_arrays", "at32_external", '{"idx": 0, "arr": []}', "0", linker_options)
	expect_success2("test_arrays", "at32_external", '{"idx": 1, "arr": []}', "0", linker_options)

	expect_success2("test_arrays", "at32_external", '{"idx": 0, "arr": [2, 3, 5, 7]}', "2", linker_options)
	# expect_success2("test_arrays", "at32_external", '{"idx": 1, "arr": [2, 3, 5, 7]}', "3", linker_options)
	expect_success2("test_arrays", "at32_external", '{"idx": 2, "arr": [2, 3, 5, 7]}', "5", linker_options)
	# expect_success2("test_arrays", "at32_external", '{"idx": 3, "arr": [2, 3, 5, 7]}', "7", linker_options)
	expect_success2("test_arrays", "at32_external", '{"idx": 4, "arr": [2, 3, 5, 7]}', "0", linker_options)
	
	expect_success2("test_arrays", "at256_external", '{"idx": "0", "arr": [2, 3, 5, 7, 11, 13, 17]}', "2", linker_options)
	# expect_success2("test_arrays", "at256_external", '{"idx": "1", "arr": [2, 3, 5, 7, 11, 13, 17]}', "3", linker_options)
	# expect_success2("test_arrays", "at256_external", '{"idx": "2", "arr": [2, 3, 5, 7, 11, 13, 17]}', "5", linker_options)
	# expect_success2("test_arrays", "at256_external", '{"idx": "3", "arr": [2, 3, 5, 7, 11, 13, 17]}', "7", linker_options)
	expect_success2("test_arrays", "at256_external", '{"idx": "4", "arr": [2, 3, 5, 7, 11, 13, 17]}', "11", linker_options)
	# expect_success2("test_arrays", "at256_external", '{"idx": "5", "arr": [2, 3, 5, 7, 11, 13, 17]}', "13", linker_options)
	expect_success2("test_arrays", "at256_external", '{"idx": "6", "arr": [2, 3, 5, 7, 11, 13, 17]}', "17", linker_options)
	expect_success2("test_arrays", "at256_external", '{"idx": "7", "arr": [2, 3, 5, 7, 11, 13, 17]}', "0", linker_options)

	## https://oeis.org/A000040/list
	abi_params = '"arr": [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97, 101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181, 191, 193, 197, 199]'
	expect_success2("test_arrays", "at256_external", '{"idx":  "0", ' + abi_params + '}', "2", linker_options)
	# expect_success2("test_arrays", "at256_external", '{"idx":  "1", ' + abi_params + '}', "3", linker_options)
	# expect_success2("test_arrays", "at256_external", '{"idx":  "2", ' + abi_params + '}', "5", linker_options)
	# expect_success2("test_arrays", "at256_external", '{"idx": "33", ' + abi_params + '}', "139", linker_options)
	# expect_success2("test_arrays", "at256_external", '{"idx": "34", ' + abi_params + '}', "149", linker_options)
	# expect_success2("test_arrays", "at256_external", '{"idx": "35", ' + abi_params + '}', "151", linker_options)
	expect_success2("test_arrays", "at256_external", '{"idx": "36", ' + abi_params + '}', "157", linker_options)
	# expect_success2("test_arrays", "at256_external", '{"idx": "37", ' + abi_params + '}', "163", linker_options)
	# expect_success2("test_arrays", "at256_external", '{"idx": "42", ' + abi_params + '}', "191", linker_options)
	# expect_success2("test_arrays", "at256_external", '{"idx": "43", ' + abi_params + '}', "193", linker_options)
	# expect_success2("test_arrays", "at256_external", '{"idx": "44", ' + abi_params + '}', "197", linker_options)
	expect_success2("test_arrays", "at256_external", '{"idx": "45", ' + abi_params + '}', "199", linker_options)

	# expect_success2("test_arrays", "atAt256_external", '{"idx": "30", ' + abi_params + ', "idy": "6"}', "157", linker_options)
	expect_success2("test_arrays", "atAt256_external", '{"idx": "30", ' + abi_params + ', "idy": "7"}', "163", linker_options)
	# expect_success2("test_arrays", "atAt256_external", '{"idx": "40", ' + abi_params + ', "idy": "2"}', "191", linker_options)

	expect_success2("test_arrays", "atAt32_external", '{"idx": "1", "arr": [2, 3, 5, 7], "idy": "2"}', "7", linker_options)
	expect_success2("test_arrays", "atAt32_external", '{"idx": "2", "arr": [2, 3, 5, 7], "idy": "1"}', "7", linker_options)

	abi_params = '"arr": [1000000007, 1000000009, 1000000021, 1000000033, 1000000087, 1000000093, 1000000097, 1000000103, 1000000123, 1000000181, 1000000207, 1000000223, 1000000241, 1000000271, 1000000289, 1000000297, 1000000321, 1000000349, 1000000363, 1000000403, 1000000409, 1000000411, 1000000427, 1000000433, 1000000439, 1000000447, 1000000453, 1000000459, 1000000483, 1000000513, 1000000531, 1000000579, 1000000607, 1000000613, 1000000637, 1000000663, 1000000711, 1000000753, 1000000787, 1000000801, 1000000829, 1000000861, 1000000871, 1000000891, 1000000901, 1000000919, 1000000931, 1000000933, 1000000993, 1000001011, 1000001021, 1000001053, 1000001087, 1000001099, 1000001137, 1000001161, 1000001203, 1000001213, 1000001237, 1000001263, 1000001269, 1000001273, 1000001279, 1000001311, 1000001329, 1000001333, 1000001351, 1000001371, 1000001393, 1000001413, 1000001447, 1000001449, 1000001491, 1000001501, 1000001531, 1000001537, 1000001539, 1000001581, 1000001617, 1000001621, 1000001633, 1000001647, 1000001663, 1000001677, 1000001699, 1000001759, 1000001773, 1000001789, 1000001791, 1000001801, 1000001803, 1000001819, 1000001857, 1000001887, 1000001917, 1000001927, 1000001957, 1000001963, 1000001969]'
	# expect_success2("test_arrays", "at32_external", '{"idx":   "0", ' + abi_params + '}', "1000000007", linker_options)
	# expect_success2("test_arrays", "at32_external", '{"idx":   "1", ' + abi_params + '}', "1000000009", linker_options)
	# expect_success2("test_arrays", "at32_external", '{"idx":   "2", ' + abi_params + '}', "1000000021", linker_options)
	expect_success2("test_arrays", "at32_external", '{"idx":  "29", ' + abi_params + '}', "1000000513", linker_options)
	# expect_success2("test_arrays", "at32_external", '{"idx":  "30", ' + abi_params + '}', "1000000531", linker_options)
	# expect_success2("test_arrays", "at32_external", '{"idx":  "31", ' + abi_params + '}', "1000000579", linker_options)
	# expect_success2("test_arrays", "at32_external", '{"idx":  "52", ' + abi_params + '}', "1000001087", linker_options)
	# expect_success2("test_arrays", "at32_external", '{"idx":  "53", ' + abi_params + '}', "1000001099", linker_options)
	expect_success2("test_arrays", "at32_external", '{"idx":  "54", ' + abi_params + '}', "1000001137", linker_options)

def testFailing():
	linker_options = "--trace"
	compile2('test_arrays', 'tests')
	expect_success2("test_arrays", "at256_external", '{"idx": "0", "arr": [2, 3, 5, 7, 11, 13, 17]}', "2", linker_options)

def testCall():
	linker_options = "--sign key1 --decode-c6"
	compile2('test_call1', 'tests')

	expect_success2('test_call1', 'constructor', '{}', '', linker_options)
	addr = '1'*64
	expect_success2('test_call1', 'send_external', '{"a": "0x' + addr + '"}', \
		'7719472615821079694904732333912527190217998977709370935963838933860875309329', linker_options)
	expect_output(r"destination : 0:1111111111111111111111111111111111111111111111111111111111111111")
	expect_output(r"body_hex: 00852d5aea")


testOld()
testOld2()
testArrays()
testCall()
#testFailing()

'''
TODO: uncomment tests when stdlib_c.tvm will support new spec
SIGN = 'key1'
compile1('hello.code', 'stdlib_c.tvm')
expect_success("hello", "", "1", "")
expect_output(r"Hello")
SIGN = None

compile1('hello.code', 'stdlib_c.tvm')
expect_success("hello", "", "1", "")
expect_output(r"Hello")
'''
cleanup()
