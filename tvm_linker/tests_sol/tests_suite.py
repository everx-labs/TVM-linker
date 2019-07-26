import os
import subprocess
import re
import unittest
import json
import time

'''
    TODO:
        - parse account state for data section:
                data:(just
                  value:(raw@^Cell 
                    x{}l
                     x{}
                    ))
                library:hme_empty))))
        - add contract02 support
'''

cfgFile_name = __file__.replace('.py','') + '.json'
script_path = os.path.dirname(os.path.abspath(__file__))
print('Script folder {}'.format(script_path))
print('Loading config file ' + cfgFile_name)
cfgFile = None
if os.access(cfgFile_name, os.R_OK):
    with open(cfgFile_name) as cfgFile_fd:
        cfgFile = cfgFile_fd.read()
        cfgFile_fd.close()
else:
    print('Config file ' + cfgFile_name + ' not found or inaccessible for reading')
    exit(-1)
if cfgFile == None:
    print('Config file ' + cfgFile_name + ' is empty')
    exit(-2)
try:
    cfg = json.loads(cfgFile)
except json.JSONDecodeError as err:
    print('Parsing config file ' + cfgFile_name + ' error:\n' + err.msg)
    exit(-3)

def runLinker(args: str):
    cmd = cfg.get('tvm_linker').get('bin_path')
    if cfg.get('tvm_linker').get('args') != None:
        cmd = cmd + ' ' + cfg.get('tvm_linker').get('args')
    cmd = cmd + ' ' + args
    _args = cmd.split(" ")
    proc = subprocess.Popen(_args, \
        cwd = cfg.get('tvm_linker').get('work_dir', './'), \
        universal_newlines=True, \
        stdout = subprocess.PIPE, \
        stderr = subprocess.STDOUT)
    return(proc)

def runLinkerCompile(contract:str, abi_json:str = None):
    res=None
    #print("Compiling {}".format(contract))
    if not(os.access(os.path.abspath(contract), os.R_OK)):
        print("Cannot access " + os.path.abspath(contract))
        return(res)
    cmd = "compile --lib " + cfg.get('tvm_linker').get('lib_path', None) + \
        " " + os.path.abspath(contract) + \
        (" --abi-json " + abi_json if abi_json!=None else "")
    # print(cmd)
    proc = runLinker(cmd)
    proc.wait()
    if proc.returncode!=0:
        err = proc.stdout.read()
        proc.stdout.close()
        print(err)
        raise Exception('Error compiling contract:')
    else:
        output = proc.stdout.read()
        proc.stdout.close()
        res = re.findall('([A-Za-z0-9]*).tvc',output)[0]
    return(res)

def runLinkerMsgInit(address:str):
    res=None
    cmd = 'message {} --init -w 0'
    proc = runLinker(cmd.format(address))
    proc.wait()
    if proc.returncode!=0:
        err = proc.stdout.read()
        proc.stdout.close()
        print(err)
        raise Exception('Error initializing message for contract')
    else:
        output = proc.stdout.read()
        proc.stdout.close()
        res = re.findall('boc file created: ([A-Za-z0-9-.]*)$',output)[0]
    return(res)

def runLinkerMsgBody(address:str, abi_json:str, abi_params:str, method:str):
    res=None
    if not(os.access(os.path.abspath(abi_json), os.R_OK)):
        return(res)
    cmd = 'message {} -w 0 --abi-json {} --abi-method {} --abi-params {}'
    cmd = cmd.format(address, os.path.abspath(abi_json), method, abi_params)
    # print(cmd)
    proc = runLinker(cmd)
    proc.wait()
    if proc.returncode!=0:
        err = proc.stdout.read()
        proc.stdout.close()
        print(err)
        raise Exception('Error preparing message body for contract address {}'.format(address))
    else:
        output = proc.stdout.read()
        # print(output)
        proc.stdout.close()
        if len(re.findall(r'boc file created: ([A-Za-z0-9-\.]*)$',output))==0:
            print(output)
            raise Exception('No boc file created for address {}'.format(address))
        res = re.findall(r'boc file created: ([A-Za-z0-9-\.]*)$',output)[0]
    return(res)

def runSDK(args:str):
    cmd = cfg.get('sdk').get('bin_path')
    if cfg.get('sdk').get('args') != None:
        cmd = cmd + ' ' + cfg.get('sdk').get('args')
    cmd = cmd + ' ' + args
    _args = cmd.split(" ")
    proc = subprocess.Popen(_args, \
        cwd = cfg.get('sdk').get('work_dir', './'), \
        universal_newlines=True, \
        stdout = subprocess.PIPE, \
        stderr = subprocess.STDOUT)
    return(proc)

def runCreateMessage(src:str, dst:str, amount:str, out_file:str):
    res=None
    cmd = '--type transfer --src {} --dst {} --value {} --out {}'
    proc = runSDK(cmd.format(src, dst, amount, os.path.abspath(out_file)))
    proc.wait()
    if proc.returncode!=0:
        err = proc.stdout.read()
        proc.stdout.close()
        print(err)
        raise Exception('Error compiling contract:')
    else:
        output = proc.stdout.read()
        proc.stdout.close()
        res = re.findall(r'BoC succsessfully saved: ([A-Za-z0-9/\-_]*.boc)$',output)[0]
    return(res)

def runTLC(args:str):
    cmd = cfg.get('tlc').get('bin_path')
    if cfg.get('tlc').get('args') != None:
        cmd = cmd + ' ' + cfg.get('tlc').get('args')
    if args!=None:
        cmd = cmd + ' ' + args
    _args = cmd.split(" ")
    proc = subprocess.Popen(_args, \
        cwd = cfg.get('tlc').get('work_dir', './'), \
        universal_newlines=True, \
        stdout = subprocess.PIPE, \
        stderr = subprocess.STDOUT)
    return(proc)

def runTLCAccount(address:str):
    res = {'result': False, 'output': None}
    cmd = '-a 0:{}'
    proc = runTLC(cmd.format(address))
    st = time.time()*1000
    ec = proc.poll()
    while (ec == None) and (time.time()*1000-st) < 1500:
        ec = proc.poll()
        time.sleep(0.1)
    if ec == None:
        print('Process {} is probably hanged. Terminating.'.format(proc.pid))
        proc.terminate()
        proc.stdout.close()
        if not(proc.poll()):
            proc.kill()
        return res
    res['output'] = proc.stdout.read()
    proc.stdout.close()
    
    # fetching address
    tmp = re.findall(r'address\:x([\da-fA-F]*)',res['output'])
    if len(tmp)>0: res['address'] = tmp[0]
    
    # fetching anycast status
    tmp = re.findall(r'anycast\:([\w]*)',res['output'])
    if len(tmp)>0: res['anycast'] = tmp[0]

    # fetching workchain
    tmp = re.findall(r'workchain_id\:([\da-fA-F]*)',res['output'])
    if len(tmp)>0: res['workchain'] = tmp[0]
    
    # fetching balance
    tmp = re.findall(r'grams:\(nanograms[\n\s]*amount:\(var_uint len:[\d]* value:([\d]*)\)\)',\
        res['output'])
    if len(tmp)>0: 
        res['balance'] = int(tmp[0])
        #print('Account {} balance {}'.format(res.get('address',None),res['balance']))

    # fetching stack
    tmp = re.findall(r'library\:hme_empty[\)]*[\n\s]*([\d\w\n\{\}\s]*)',res['output'])
    if len(tmp)>0: res['stack'] = tmp[0].splitlines()

    # return result
    return res

def runTLCFile(boc_file:str):
    res = {'result': False, 'output': None}
    if not(os.access(os.path.abspath(boc_file), os.R_OK)):
        return(res)
    cmd = '-f {}'
    proc = runTLC(cmd.format(boc_file))
    st = time.time()*1000
    ec = proc.poll()
    while ec==None and (time.time()*1000-st)<10000:
        ec = proc.poll()
        time.sleep(0.1)
    if ec == None:
        print('Process {} is probably hanged. Terminating.'.format(proc.pid))
        proc.terminate()
        proc.stdout.close()
        if not(proc.poll()):
            proc.kill()
        return res
    res['output'] = proc.stdout.read()
    proc.stdout.close()
    res['result'] = True
    return(res)

def waitFor(function, args, timeout, condition, re_flags=re.I):
    sdt = int(round(time.time()*1000))
    res = {'result': False, 'output': None}
    while (res['output']==None or len(re.findall(condition, res['output']))<1) \
        and (int(round(time.time()*1000))-sdt)<timeout:
        res = function(*args)
        time.sleep(0.25)
    if len(re.findall(condition, res['output'], re_flags))==0:
        print('Looking for:\n{}\nOutput:\n{}'.format(condition, res['output']))
        raise Exception('Unable to find condition string in output')
    else:
        res['result'] = True
    return(res)        

def waitForBalanceInRange(account, min_value, max_value, timeout):
    if max_value<min_value:
        _min = max_value
        _max = min_value
    else:
        _min = min_value
        _max = max_value
    sdt = int(round(time.time()*1000))
    res = runTLCAccount(account)
    while res.get('balance')!=None and (res.get('balance')<_min or res.get('balance')>_max) \
        and (int(round(time.time()*1000))-sdt)<timeout:
        res = runTLCAccount(account)
    if res.get('balance')!=None and (res.get('balance')<_min or res.get('balance')>_max):
        raise Exception('Balance ' + res.get('balance') + ' not in specified range')
    return(res)        

def waitForStackChanged(account, timeout, prev_stack=None):
    sdt = int(round(time.time()*1000))
    init_stack=None
    if prev_stack==None:
        res = runTLCAccount(account)
        init_stack = res.get('stack')
    else:
        init_stack = prev_stack
    # waiting that stack changes
    stack_eq = True
    c_stack = init_stack
    while c_stack!=None and stack_eq\
        and (int(round(time.time()*1000))-sdt)<timeout:
        res = runTLCAccount(account)
        c_stack = res.get('stack')
        stack_eq = init_stack!=None and c_stack!=None
        # if both stacks not None and has the same size compare each element
        if stack_eq and len(c_stack) == len(init_stack):
            for i in range(len(c_stack)):
                stack_eq = stack_eq and c_stack[i]==init_stack[i]
                if not(stack_eq):
                    break
        else:
            stack_eq = False
    if stack_eq:
        raise Exception('Stack hasn\'t been changed during timeout')
    
    # waiting to make sure that it is the final changes
    init_stack = c_stack
    stack_eq = True
    sdt = int(round(time.time()*1000))
    while c_stack!=None and stack_eq\
        and (int(round(time.time()*1000))-sdt)<3000:
        res = runTLCAccount(account)
        c_stack = res.get('stack')
        stack_eq = init_stack!=None and c_stack!=None
        # if both stacks not None and has the same size compare each element
        if stack_eq and len(c_stack) == len(init_stack):
            for i in range(len(c_stack)):
                stack_eq = stack_eq and c_stack[i]==init_stack[i]
                if not(stack_eq):
                    init_stack = c_stack
                    sdt = int(round(time.time()*1000))
                    break
        else:
            init_stack = c_stack
            sdt = int(round(time.time()*1000))
            stack_eq = True
    
    # return result
    return(res)        

class SoliditySuite(unittest.TestCase):
    def setUp(self):
        print('\nSetting up')
        self.cfg = cfg
        self.assertNotEqual(self.cfg.get('node', None), None, 'No node config provided')
        wd = self.cfg['node'].get('work_dir',None)
        if wd != None: 
            self.assertTrue(os.access(wd, os.R_OK), 'No node workdir found')
            os.chdir(wd)
        subprocess.call('pkill ton-node', shell=True)
        subprocess.call('rm -f ./log/output.log', shell=True)
        subprocess.call('rm -rf ./workchains', shell=True)
        cmd = self.cfg['node'].get('cmd')
        self.node = None
        self.node = subprocess.Popen(cmd, shell=True)
        
        # give some time for node to start
        time.sleep(1)
        
        os.chdir(script_path)
        subprocess.call('rm -f *.tvc *.boc *.tmp', shell=True)
        
    def tearDown(self):
        print('\nFinishing')
        if self.node!=None:
            self.node.terminate()
            self.node.wait()
        subprocess.call('pkill ton-node', shell=True)
        
    def deployContract(self, contractName:str, contract_abi:str, amount: str):
        address = runLinkerCompile(contractName, contract_abi)
        self.assertNotEqual(address,None, \
            'Contract {} hasn\'t been compiled'.format(contractName))
        #print('Contract {} address: {}'.format(contractName, address))
        msginit = runLinkerMsgInit(address)
        self.assertNotEqual(msginit, None, \
            'No msg init boc file created for contract {}'.format(contractName))
        #print('Contract {} message init file: {}'.format(contractName, msginit))
        
        msgfile = runCreateMessage('0' * 64, address, amount, \
            './sendmoney{}.boc'.format(address))
        self.assertEqual(msgfile, os.path.abspath('./sendmoney{}.boc'.format(address)),\
            'Expected message file for contract {} wasn\'t been created'.format(contractName))
        #print('Created message file for contract {}:'.format(contractName), msgfile)

        # fetching state of zero account to make sure node is up
        waitFor(runTLCAccount, ["0" * 64], 5000, r'state:\(account_active')

        waitFor(runTLCFile, [msgfile], 5000, r'external message status is 1')
        waitFor(runTLCAccount, [address], 5000, r'state:account_uninit')
        
        waitFor(runTLCFile, [msginit], 5000, r'external message status is 1')
        waitFor(runTLCAccount, [address], 5000, r'state:\(account_active')
        
        return address
    
    def test_01(self):
        address = self.deployContract('contract01.code', 'contract01.abi.json','1000000')
    
        msgbody = runLinkerMsgBody(address, 'contract01.abi.json', '{"a":"0x1234"}', 'main_external')
        self.assertNotEqual(msgbody, None, 'No msg body boc file created')
        
        tmp = waitFor(runTLCAccount, [address], 5000, r'state:\(account_active')
        self.assertEqual(tmp['stack'][len(tmp['stack'])-1] \
            if len(tmp.get('stack',None))>0 else None, " x{4_}")

        waitFor(runTLCFile, [msgbody], 5000, r'external message status is 1')

        tmp = waitFor(runTLCAccount,[address], 5000, \
            r'\{D0000001234\}')
        self.assertEqual(tmp['stack'][len(tmp['stack'])-1] if len(tmp.get('stack',None))>0 \
            else None, "  x{D0000001234}")
        
    def test_02(self):
        # prepare contract a
        address1 = self.deployContract('contract02-a.code', 'contract02-a.abi.json','1000000')
        
        # prepare contract b
        address2 = self.deployContract('contract02-b.code', 'contract02-b.abi.json','1000000')
        
        # prepare message body for contract a
        msgbody = runLinkerMsgBody(address1, 'contract02-a.abi.json', '{"anotherContract":"0x' + \
            address2 + '"}', 'method_external')

        # checking initial account state
        waitFor(runTLCAccount,[address1], 5000, r'(state:\(account_active)')
        waitFor(runTLCAccount,[address2], 5000, r'(state:\(account_active)')
        
        # sending body to node
        waitFor(runTLCFile, [msgbody], 5000, r'external message status is 1')

        waitFor(runTLCAccount,[address1], 5000, \
            r'x\{D000000000000000000000000000000000000000000000000000000000000000001\}')
        waitFor(runTLCAccount,[address2], 5000, \
            r'x\{D000000000000000101\}')
        
    def test_03(self):
        # prepare contract a
        address1 = self.deployContract('contract03-a.code', 'contract03-a.abi.json','1000000')
        
        # prepare contract b
        address2 = self.deployContract('contract03-b.code', 'contract03-b.abi.json','1000000')
        
        # prepare message body for contract a
        msgbody = runLinkerMsgBody(address1, 'contract03-a.abi.json', '{"anotherContract":"0x' + \
            address2 + '"}', 'method_external')

        # checking initial account state
        waitFor(runTLCAccount,[address1], 5000, r'(state:\(account_active)')
        waitFor(runTLCAccount,[address2], 5000, r'(state:\(account_active)')
        
        # sending body to node
        waitFor(runTLCFile, [msgbody], 5000, r'external message status is 1')

        waitFor(runTLCAccount,[address1], 5000, \
            r'x\{D000000000000000000000000000000000000000000000000000000000000000001\}')
        str2expect = r'x\{D00%s\}' % address1
        waitFor(runTLCAccount,[address2], 5000, str2expect)
        
    def test_04(self):
        # prepare contract a
        address1 = self.deployContract('contract04-a.code', 'contract04-a.abi.json','10000000')
        
        # prepare contract b
        address2 = self.deployContract('contract04-b.code', 'contract04-b.abi.json','10000000')
        
        # prepare message body for contract a
        msgbody = runLinkerMsgBody(address1, 'contract04-a.abi.json', '{"anotherContract":"0x' + \
            address2 + '","amount":"5000000"}', 'method_external')
        
        # checking initial account state
        waitFor(runTLCAccount,[address1], 5000, r'state:\(account_active')
        waitFor(runTLCAccount,[address2], 5000, r'state:\(account_active')
        
        # sending body to node
        waitFor(runTLCFile, [msgbody], 5000, r'external message status is 1')

        # checking account balance changes
        waitForBalanceInRange(address1, 14700000, 15100000, 5000)
        waitForBalanceInRange(address2, 4700000, 5300000, 5000)

    def test_05(self):
        # prepare contract a
        address1 = self.deployContract('contract05-a.code', 'contract05-a.abi.json','10000000')
        
        # prepare contract b
        address2 = self.deployContract('contract05-b.code', 'contract05-b.abi.json','10000000')
        
        # prepare message body for contract a
        msgbody = runLinkerMsgBody(address1, 'contract05-a.abi.json', '{"anotherContract":"0x' + \
            address2 + '","x":"257"}', 'method_external')

        # checking initial account state
        waitFor(runTLCAccount,[address1], 5000, r'state:\(account_active')
        waitFor(runTLCAccount,[address2], 5000, r'state:\(account_active')
        
        # sending body to node
        waitFor(runTLCFile, [msgbody], 5000, r'external message status is 1')

        # checking account balance changes
        waitFor(runTLCAccount,[address2], 5000, r'x\{D000101\}')
        waitFor(runTLCAccount,[address1], 5000, r'x\{D000000000000000000000000000000000000000000000000000000000000001010\}').get('stack')

    def test_06(self):
        # prepare contract a
        address1 = self.deployContract('contract06-a.code', 'contract06-a.abi.json','10000000')
        
        # prepare contract b
        address2 = self.deployContract('contract06-b.code', 'contract06-b.abi.json','10000000')
        
        # prepare message body for contract1
        msgbody1 = runLinkerMsgBody(address1, 'contract06-a.abi.json', '{"anotherContract":"0x' + \
            address2 + '","amount":"0x12345678"}', 'setAllowance_external')
        
        # prepare message body for contract2
        msgbody2 = runLinkerMsgBody(address2, 'contract06-b.abi.json', '{"bank":"0x' + \
            address1 + '"}', 'getMyCredit_external')
        
        waitFor(runTLCAccount,[address1], 5000, r'state:\(account_active')

        # checking initial account state
        a1 = waitFor(runTLCAccount,[address1], 5000, r'state:\(account_active')
        a2 = waitFor(runTLCAccount,[address2], 5000, r'state:\(account_active')
        
        # sending contract1 message body to node
        waitFor(runTLCFile, [msgbody1], 5000, r'external message status is 1')

        # checking account stack changes
        waitForStackChanged(address1, 5000, a1.get('stack'))

        # sending contract2 message body to node
        waitFor(runTLCFile, [msgbody2], 5000, r'external message status is 1')

        # checking account stack changes
        tmp = waitForStackChanged(address2, 5000, a2.get('stack'))
        last_rec = tmp['stack'][len(tmp['stack'])-1] if len(tmp.get('stack'))>0 else None
        self.assertEqual(last_rec.strip(),\
            'x{D000000000012345678}',\
            'Unexpected allowance value for contract2')

    def test_07(self):
        # prepare contract a
        address1 = self.deployContract('contract07-a.code', 'contract07-a.abi.json','1000000')
        
        # prepare contract b
        address2 = self.deployContract('contract07-b.code', 'contract07-b.abi.json','1000000')
        
        # prepare message body for contract a
        msgbody = runLinkerMsgBody(address1, 'contract07-a.abi.json', '{"anotherContract":"0x' + \
            address2 + '"}', 'method_external')

        # checking initial account state
        waitFor(runTLCAccount,[address1], 5000, r'(state:\(account_active)').get('balance')
        b2 = waitFor(runTLCAccount,[address2], 5000, r'(state:\(account_active)').get('balance')
        
        # sending body to node
        waitFor(runTLCFile, [msgbody], 5000, r'external message status is 1')

        s1 = waitForStackChanged(address1, 5000).get('stack')
        s1 = re.findall(r'D0([0-9]*)',s1[len(s1)-1])[0]
        v1 = int(s1, base=16)
        self.assertEqual(v1, 1, 'Unexpected stack value')
        s2 = runTLCAccount(address2).get('stack')
        s2 = re.findall(r'D0([0-9A-Z]*)',s2[len(s2)-1])[0]
        v2 = int(s2, base=16)
        self.assertTrue(v2>b2 and v2<1000000, 'Unexpected stack balance value')
        
if __name__ == '__main__':
    unittest.main()