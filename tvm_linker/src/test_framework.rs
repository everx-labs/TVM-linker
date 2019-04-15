use std::convert::Into;
use std::ops::Range;
use std::sync::Arc;
use tvm::assembler;
use tvm::executor;
use tvm::assembler::CodePage0;
use tvm::assembler::Writer;
use tvm::executor::Engine;
pub use tvm::stack::{
    ContinuationData, 
    IntegerData, 
    SaveList, 
    SliceData,
    CellData,
    Stack, 
    StackItem, 
};
pub use tvm::types::Exception;
pub use tvm::types::ExceptionCode;
use tvm::logger;

pub type Bytecode = SliceData;

pub struct TestCaseInputs {
    code: String,
    registers: SaveList,
    stack: Stack,
    refs: Vec<SliceData>
}

impl TestCaseInputs {
    pub fn new(code: &str) -> TestCaseInputs {
        logger::init();
        TestCaseInputs {
            code: code.to_string(),
            registers: SaveList::new(),
            stack: Stack::new(),
            refs: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_refs(mut self, refs: Vec<SliceData>) -> TestCaseInputs {
        self.refs = refs;
        self
    }


    #[allow(dead_code)]
    pub fn with_root_data(self, root_data: Arc<CellData>) -> TestCaseInputs {
        self.with_ctrl(4, StackItem::Cell(root_data))
    }

    #[allow(dead_code)]
    pub fn with_stack(mut self, stack: Stack) -> TestCaseInputs {
        self.stack = stack;
        self
    }

    #[allow(dead_code)]
    pub fn with_ctrl(mut self, ctrl: usize, mut item: StackItem) -> TestCaseInputs {
        self.registers.put(ctrl, &mut item)
            .expect("test arguments must be valid");
        self
    }
}

impl Into<TestCase> for TestCaseInputs {
    fn into(self) -> TestCase {
        let refs = self.refs.clone();
        TestCase::new(self, refs)
    }
}

pub struct TestCase {
    executor: Option<Engine>,
    compilation_result: Result<Bytecode, assembler::CompileError>,
    execution_result: Option<Exception>,
}

impl TestCase {
    pub fn new(args: TestCaseInputs, references: Vec<SliceData>) -> TestCase {
        let compilation_result = assembler::Engine::<CodePage0>::new().compile(&args.code)
            .map(|code| code.finalize());
        let executor_option: Option<Engine>;
        let execution_result: Option<Exception>;
        match compilation_result {
            Ok(ref code) => {
                let mut code = code.clone();
                for reference in references {
                    code.append_reference(reference);
                }
                let mut registers = args.registers.clone();
                registers.put(3, &mut StackItem::Continuation(
                    ContinuationData::withdraw_from(&mut code.clone().into())
                )).unwrap();
                let mut executor = executor::Engine::new()
                    .setup(code, registers, args.stack.clone())
                    .unwrap_or_else(|e| panic!("Cannot setup engine, error {}", e));
                if cfg!(feature = "verbose") {
                    executor.set_trace(Engine::TRACE_CODE);
                }
                execution_result = executor.execute();
                executor_option = Some(executor);
            }
            Err(ref _e) => {
                execution_result = None;
                executor_option = None;
            }
        }
        TestCase {
            executor: executor_option,
            compilation_result: compilation_result,
            execution_result: execution_result,
        }
    }

    pub fn get_root(&self) -> Option<Arc<CellData>> {
        if let Some(ref eng) = self.executor {
            match eng.get_root() {
                StackItem::Cell(c) => Some(c),
                _ => None,
            }
        } else {
            None
        }
    }

}

pub trait Expects {
    fn expect_bytecode(self, bytecode: Vec<u8>) -> TestCase;
    fn expect_bytecode_extended(self, bytecode: Vec<u8>, message: Option <&str>) -> TestCase;
    fn expect_compilation_failure(self, error: assembler::CompileError) -> TestCase;
    fn expect_compilation_failure_extended(self, error: assembler::CompileError, message: Option <&str>) -> TestCase;
    fn expect_stack(self, stack: &Stack) -> TestCase;
    fn expect_stack_extended(self, stack: &Stack, message: Option<&str>) -> TestCase;
    fn expect_empty_stack(self) -> TestCase;
    fn expect_int_stack(self, stack_contents: &[i32]) -> TestCase;
    fn expect_item(self, stack_item: StackItem) -> TestCase;
    fn expect_item_extended(self, stack_item: StackItem, message: Option<&str>) -> TestCase;
    fn expect_success(self) -> TestCase;
    fn expect_success_extended(self, message: Option <&str>) -> TestCase;
    fn expect_ctrl(self, ctrl: usize, item: &StackItem) -> TestCase;
    fn expect_ctrl_extended(self, ctrl: usize, item: &StackItem, message: Option<&str>) -> TestCase;
    fn expect_failure(self, exception_code: ExceptionCode) -> TestCase;
    fn expect_custom_failure(self, custom_code: u16) -> TestCase;
    fn expect_custom_failure_extended<F : Fn(&Exception) -> bool>(self, op: F, exc_name: &str, message: Option <&str>) -> TestCase;
    fn expect_failure_extended(self, exception_code: ExceptionCode, message: Option <&str>) -> TestCase;
    fn expect_root_data(self, cell: Arc<CellData>) -> TestCase;

    fn expect_print_stack(self) -> TestCase;
}

impl<T> Expects for T 
where
    T: Into<TestCase>
{    
    #[allow(dead_code)]
    fn expect_bytecode(self, bytecode: Vec<u8>) -> TestCase {
        self.expect_bytecode_extended(bytecode, None)
    }

    #[allow(dead_code)]
    fn expect_bytecode_extended(self, bytecode: Vec<u8>, message: Option <&str>) -> TestCase {
        let inputcode = SliceData::new(bytecode);
        let test_case: TestCase = self.into();
        match test_case.compilation_result {
            Ok(ref selfcode) => {
                let mut bytevec = vec![];
                let mut selfcode = selfcode.clone();
                while selfcode.remaining_bits() > 0 {
                    bytevec.append(&mut selfcode.withdraw_data());
                    assert_eq!(bytevec.pop(), Some(0x80)); // remove completion tag 0x80
                    if selfcode.remaining_references() > 0 {
                        selfcode = selfcode.drain_reference().into();
                    }
                }
                bytevec.push(0x80);
                let selfcode = SliceData::new(bytevec);
                if !selfcode.eq(&inputcode) {
                    match message {
                        Some(msg) => panic!(
                            "{}Bytecode did not match:\n Expected: <{:x?}>\n But was: <{:x?}>",
                            msg, inputcode, selfcode),
                        None => panic!(
                            "Bytecode did not match:\n Expected: <{:x?}>\n But was: <{:x?}>",
                            inputcode, selfcode),
                    }
                };
            },
            Err(e) => {
                match message {
                    Some(msg) => panic!("{}{}", msg, e),
                    None => panic!(e),
                }
            }
        }
        test_case
    }

    #[allow(dead_code)]
    fn expect_compilation_failure(self, error: assembler::CompileError) -> TestCase {
        self.expect_compilation_failure_extended(error, None)
    }
    
    #[allow(dead_code)]
    fn expect_compilation_failure_extended(self, error: assembler::CompileError, message: Option <&str>) -> TestCase {
        let test_case = self.into();
        match message {
            None => {
                let actual = &test_case.compilation_result.clone().expect_err("Error expected");
                assert_eq!(
                    &error, actual,
                    "Expected (left): <{}>, but was (right): <{}>.",
                    &error, &actual
                )
            },
            Some(msg) => {
                let actual = &test_case.compilation_result.clone().expect_err(&(msg.to_string() + ". Error expected"));
                assert_eq!(
                    &error, actual,
                    "{}\nExpected (left): <{}>, but was (right): <{}>.",
                    msg, &error, &actual
                )
            },
        }
        test_case
    }

    #[allow(dead_code)]
    fn expect_stack(self, stack: &Stack) -> TestCase {
        self.expect_stack_extended(stack, None)
    }
    
    fn expect_stack_extended(self, stack: &Stack, message: Option<&str>) -> TestCase {
        let test_case = self.into();
        match test_case.executor {
            Some(ref executor) => {
                match test_case.execution_result {
                    None => {
                        if !executor.eq_stack(stack) {
                            match message {
                                Some(msg) => print!("{}",msg),
                                None => {},
                            }
                            logger::info(format_args!("Expected stack: \n{}", stack));
                            executor.print_info_stack("Actual Stack:");
                            executor.assert_stack(stack);
                        }
                    }
                    Some(ref e) => {
                        print_failed_detail_extended(&test_case, e, message);
                        panic!("Execution error: {}", e);
                    }
                };
            },
            None => abort_if_no_executor(&test_case, message)
        };
        test_case
    }

    #[allow(dead_code)]
    fn expect_empty_stack(self) -> TestCase {
        self.expect_stack(&Stack::new())
    }

    #[allow(dead_code)]
    fn expect_int_stack(self, stack_contents: &[i32]) -> TestCase {
        let test_case = self.into();
        let mut stack = Stack::new();
        for element in stack_contents {
            let item = IntegerData::from(*element).unwrap();
            stack.push(StackItem::Integer(item));
        }
        test_case.expect_stack(&stack)
    }

    #[allow(dead_code)]
    fn expect_item(self, stack_item: StackItem) -> TestCase {
        self.expect_item_extended(stack_item, None)
    }

    #[allow(dead_code)]
    fn expect_item_extended(self, stack_item: StackItem, message: Option<&str>) -> TestCase {
        self.expect_stack_extended(Stack::new().push(stack_item), message)
    }

    #[allow(dead_code)]
    fn expect_success(self) -> TestCase {
       self.expect_success_extended(None)
    }

    #[allow(dead_code)]
    fn expect_success_extended(self, message: Option <&str>) -> TestCase {
        let test_case = self.into();
        match test_case.executor {
            Some(ref executor) => {
                print_stack(&test_case, &executor);
                match test_case.execution_result {
                    None => {}
                    Some(ref e) => {
                        match message {
                            None => {
                                print_failed_detail_extended(&test_case, e, message);
                                panic!("Execution error: {}", e);
                            }
                            Some(msg) => {
                                print_failed_detail_extended(&test_case, e, message);
                                panic!("{}\nExecution error: {}", msg, e);
                            }
                        }
                    }
                };
            },
            None => abort_if_no_executor(&test_case, message)
        };
        test_case
    }

    #[allow(dead_code)]
    fn expect_ctrl(self, ctrl: usize, item: &StackItem) -> TestCase {
        self.expect_ctrl_extended(ctrl, item, None)
    }

    #[allow(dead_code)]
    fn expect_ctrl_extended(self, ctrl: usize, item: &StackItem, message: Option<&str>) -> TestCase {
        let test_case = self.into();
        match test_case.executor {
            Some(ref executor) => {
                match test_case.execution_result {
                    None => {
                        executor.assert_ctrl(ctrl, item);
                    }
                    Some(ref e) => {
                        print_failed_detail_extended(&test_case, e, message);
                        panic!("Execution error: {}", e);
                    }
                };
            },
            None => abort_if_no_executor(&test_case, message)
        };
        test_case
    }

    #[allow(dead_code)]
    fn expect_failure(self, exception_code: ExceptionCode) -> TestCase {
        self.expect_failure_extended(exception_code, None)
    }
    
    fn expect_custom_failure_extended<F : Fn(&Exception) -> bool>(self, op: F, exc_name: &str, message: Option <&str>) -> TestCase {
        let test_case = self.into();
        match test_case.executor {
            Some(ref executor) => {
                match test_case.execution_result {
                    None => {
                        logger::info(format_args!("Expected failure, however execution succeeded."));
                        print_stack(&test_case, &executor);
                        match message {
                            None => panic!("Expected failure, however execution succeeded."),
                            Some(msg) => panic!("{}.\nExpected failure, however execution succeeded.",msg),
                        }
                    }
                    Some(ref e) => {
                        if op(e) {
                            match message {
                                None => {
                                    logger::info(format_args!(
                                        "Expected exception {}, but was {}",
                                        exc_name, e));
                                    panic!("Non expected exception.");
                                }
                                Some(msg) => {
                                   logger::info(format_args!("{}. Expected exception {}, but was {}",
                                        exc_name, e, msg));
                                    panic!("{}. Non expected exception.", msg);
                                }
                            }
                        }
                    }
                };
            },
            None => abort_if_no_executor(&test_case, message)
        }
        test_case
    }

    #[allow(dead_code)]
    fn expect_custom_failure(self, custom_code: u16) -> TestCase {
        self.expect_custom_failure_extended(|e| e.number != custom_code as usize, "custom exception", None)
    }

    #[allow(dead_code)]
    fn expect_failure_extended(self, exception_code: ExceptionCode, message: Option <&str>) -> TestCase {
       self.expect_custom_failure_extended(|e| e.code != exception_code, exception_code.message(), message)
    }

    fn expect_root_data(self, cell: Arc<CellData>) -> TestCase {
        self.expect_ctrl(4, &StackItem::Cell(cell))
    }

    fn expect_print_stack(self) -> TestCase {
        let test_case = self.into();
        match test_case.executor {
            Some(ref executor) => {
                executor.print_info_stack("Resulting stack state");
            }
            None => abort_if_no_executor(&test_case, None)
        }
        test_case
    }
}

pub fn test_case(code: &str) -> TestCaseInputs {
    TestCaseInputs::new(code)
}

fn abort_if_no_executor(case: &TestCase, message: Option <&str>) {
    match message {
        Some(msg) => panic!("{}No executor was created, because of bytecode compilation error {}", 
            msg, case.compilation_result.clone().err().unwrap()),
        None => panic!("No executor was created, because of bytecode compilation error {}", 
            case.compilation_result.clone().err().unwrap()),
    }
}

fn print_stack(case: &TestCase, executor: &Engine) {
    if !case.execution_result.is_none() || cfg!(feature = "verbose") {
        logger::info(format_args!("Post-execution:\n"));
        executor.print_info_stack("Post-execution stack state");
        executor.print_info_ctrls();
    }
}

#[allow(dead_code)]
fn print_failed_detail(case: &TestCase, exception: &Exception) {
    print_failed_detail_extended(case, exception, None)
}

fn print_failed_detail_extended(case: &TestCase, exception: &Exception, message: Option <&str>) {
    match message {
        Some(ref msg) => logger::info(format_args!(
            "{} failed with {}.\nBytecode: {:x?}\n",
            msg, exception, case.compilation_result
            )),
        None => logger::info(format_args!(
            "failed with {}.\nBytecode: {:x?}\n",
            exception, case.compilation_result
            )),
    }
}

#[allow(dead_code)]
pub fn test_case_with_ref(code: &str, reference: SliceData) -> TestCaseInputs {
    TestCaseInputs::new(code).with_refs(vec![reference])
}

#[allow(dead_code)]
pub fn generate_code_push_range(range: Range<usize>) -> String {
    let mut buf = String::new();
    for i in range {
        buf.push_str(&("PUSHINT ".to_owned() + &i.to_string() + "\n"));
    }

    return buf;
}

