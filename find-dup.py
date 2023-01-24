# Find duplicate fragments of linear sequences of code

# Sample invocation:
# 1. extract code boc into the input.boc with tvm_linker decode
# 2. tvm_linker disasm text --raw input.boc >input.code
# 3. python3 find-dup.py input.code

import sys

if len(sys.argv) != 2:
  print("Usage: find-dup.py input.code")
  exit(1)

size_low = 5
size_high = 101
size_width = len(str(size_high))

file = open(sys.argv[1], "r")
lines = file.readlines()
lines = [line.strip() for line in lines]
width = len(str(len(lines)))

def is_valid(pattern):
  for line in pattern:
    s = line.split(";")[0].strip()
    if s.endswith("{") or s.startswith("}"):
      return False
  return True

for n in range(size_low, size_high):
  matches = {}

  for i in range(len(lines) - n + 1):
    pattern = lines[i:i + n]
    if not is_valid(pattern):
      continue
    for j in range(i + 1, len(lines) - n + 1):
      fragment = lines[j:j + n]
      if fragment == pattern:
        if i not in matches:
          matches.update({i: []})
        l = matches[i]
        l.append(j)

  for i, js in matches.items():
    if len(js) > 1:
      s = "{n:0{w1}}: {i:0{w2}}".format(n = n, w1 = size_width, i = i, w2 = width)
      for j in js:
        s += " {j:0{w}}".format(j = j, w = width)
      print(s)
