use revm_interpreter::{
    CallInputs, CreateInputs, Gas, Host, InstructionResult, Interpreter, SelfDestructResult,
};
use revm_primitives::{Bytecode, Bytes, Env, Spec, SpecId, B160, B256, U256};

#[derive(Default)]
pub(crate) struct DummyHost(pub Env);
pub(crate) struct DummySpec;

impl Spec for DummySpec {
    const SPEC_ID: SpecId = SpecId::LATEST;
}

impl Host for DummyHost {
    fn step(&mut self, _interpreter: &mut Interpreter, _is_static: bool) -> InstructionResult {
        println!("step");
        InstructionResult::Continue
    }

    fn step_end(
        &mut self,
        _interpreter: &mut Interpreter,
        _is_static: bool,
        _ret: InstructionResult,
    ) -> InstructionResult {
        println!("step end");
        InstructionResult::Continue
    }

    fn env(&mut self) -> &mut Env {
        println!("env");
        &mut self.0
    }

    fn load_account(&mut self, _address: B160) -> Option<(bool, bool)> {
        println!("load account");
        None
    }

    fn block_hash(&mut self, _number: U256) -> Option<B256> {
        println!("block hash");
        None
    }

    fn balance(&mut self, _address: B160) -> Option<(U256, bool)> {
        println!("balance");
        None
    }

    fn code(&mut self, _address: B160) -> Option<(Bytecode, bool)> {
        println!("code");
        None
    }

    fn code_hash(&mut self, _address: B160) -> Option<(B256, bool)> {
        println!("code hash");
        None
    }

    fn sload(&mut self, _address: B160, _index: U256) -> Option<(U256, bool)> {
        println!("sload");
        None
    }

    fn sstore(
        &mut self,
        _address: B160,
        _index: U256,
        _value: U256,
    ) -> Option<(U256, U256, U256, bool)> {
        println!("sstore");
        None
    }

    fn log(&mut self, _address: B160, _topics: Vec<B256>, _data: Bytes) {
        println!("log")
    }

    fn selfdestruct(&mut self, _address: B160, _target: B160) -> Option<SelfDestructResult> {
        println!("self destruct");
        None
    }

    fn create(
        &mut self,
        _inputs: &mut CreateInputs,
    ) -> (InstructionResult, Option<B160>, Gas, Bytes) {
        println!("create");
        (
            InstructionResult::Continue,
            None,
            Gas::new(0),
            Bytes::default(),
        )
    }

    fn call(&mut self, _input: &mut CallInputs) -> (InstructionResult, Gas, Bytes) {
        println!("call");
        (InstructionResult::Continue, Gas::new(0), Bytes::new())
    }
}
