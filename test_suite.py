import subprocess
from enum import Enum
import inspect

TVM_LINKER_PATH = "./target/release/tvm_linker"
STDLIB_SOL = "./tests/test_stdlib_sol.tvm"
TRASH = "trash"
TESTS = "tests"


class CMD(Enum):
    COMPILE = "compile"


def runcmd(cmd: str) -> str:
    print(f"running cmd: \"{cmd}\"")
    return subprocess.check_output(cmd, shell=True).decode()


def runlnk(op: CMD, input: str = "") -> str:
    return runcmd(f"{TVM_LINKER_PATH} {op.value} {input}")


def prntme():
    print(f"-------- run {inspect.stack()[1].function} --------")


def test_compile():
    prntme()

    smc = "mycode"
    cmd = f"--lib {STDLIB_SOL} ./{TESTS}/{smc}.code -o ./{TRASH}/{smc}.tvc.boc"
    res = runlnk(CMD.COMPILE, cmd)

    print(f"\n{res}")


def main():
    test_compile()


if __name__ == "__main__":
    main()
