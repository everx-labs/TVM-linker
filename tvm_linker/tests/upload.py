#!/usr/bin/python2.7

import os
import re
import subprocess
import sys

TMP_FN = "upload.tmp"
LITE_CLIENT_DIR = "../../../build-tlc"
CLIENT_CMD = "{0}/test-lite-client -C {0}/ton-lite-client-test1.config.json".format(LITE_CLIENT_DIR)
TVM_LINKER_EXE = "../../../TVM-linker/tvm_linker/target/debug/tvm_linker"
TESTGIVER_ADDR = "538fa7cc24ff8eaa101d84a5f1ab7e832fe1d84b309cdfef4ee94373aac80f7d"
CONTRACT_ADDR = None
FIFT_CMD = "{0}/crypto/fift -I ../../../Telegram-Lite-Client/crypto/fift".format(LITE_CLIENT_DIR)

# Part 0. Reading address

proc = subprocess.Popen([TVM_LINKER_EXE, sys.argv[1], sys.argv[2], '--init', '--message'], stdout=subprocess.PIPE)
data = proc.communicate()
for line in data:
	if not line: continue
	print line.strip()

	g = re.search ("Address: ([0-9a-z]+)", line)
	if g:
		CONTRACT_ADDR = g.group(1)
proc.wait()

if not CONTRACT_ADDR:
	print "No contract address!"
	sys.exit()

# Part 1. Reading seq. no


if os.path.exists(TMP_FN):
	os.remove(TMP_FN)

cmd = "{0} -a {1} -l {2}".format(CLIENT_CMD, TESTGIVER_ADDR, TMP_FN);
print cmd
os.system(cmd)

lines = open(TMP_FN, "r").readlines();

seqno = None

for l in lines:
	m = re.search("x\{(........)\}", l)
	if m:
		seqno = m.group(1)

if not seqno:
	print "Wrong format!"
	sys.exit()

# Part 2. Preparing testgiver.fif

testgiver = open("testgiver.pat")
testgiver_fif = open("testgiver.fif","wt")
for l in testgiver:
	l1 = re.sub(r"\$addr\$", "0x"+CONTRACT_ADDR, l.strip())
	l2 = re.sub(r"\$seqno\$", "0x"+seqno, l1)
	testgiver_fif.write(l2+"\n")
testgiver_fif.close()

cmd = "{0} {1}".format(FIFT_CMD, "testgiver.fif")
print cmd
os.system(cmd)
