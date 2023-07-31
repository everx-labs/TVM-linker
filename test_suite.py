from dataclasses import dataclass
import subprocess
import argparse
from enum import Enum
import inspect


@dataclass
class Config:
    TVM_LINKER_PATH: str
    STDLIB_SOL: str
    TRASH: str
    TESTS: str


CFG = Config(
    TVM_LINKER_PATH="./target/release/tvm_linker",
    STDLIB_SOL="./tests/test_stdlib_sol.tvm",
    TRASH="trash",
    TESTS="tests"
)


class CMD(Enum):
    COMPILE = "compile"


def runcmd(cmd: str) -> str:
    print(f"running cmd: \"{cmd}\"")
    return subprocess.check_output(cmd, shell=True).decode()


def runlnk(op: CMD, input: str = "") -> str:
    print("CFG.TVM_LINKER_PATH: ", CFG.TVM_LINKER_PATH)
    return runcmd(f"{CFG.TVM_LINKER_PATH} {op.value} {input}")


def prntme():
    print(f"-------- run {inspect.stack()[1].function} --------")


def test_compile():
    prntme()

    smc = "mycode"
    cmd = f"--lib {CFG.STDLIB_SOL} ./{CFG.TESTS}/{smc}.code -o ./{CFG.TRASH}/{smc}.tvc.boc"
    res = runlnk(CMD.COMPILE, cmd)

    print(f"\n{res}")


def main():
    args_parser = argparse.ArgumentParser()
    args_parser.add_argument("--linker-path")
    args = args_parser.parse_args()

    if args.linker_path:
        CFG.TVM_LINKER_PATH = args.linker_path

    test_compile()


if __name__ == "__main__":
    main()
