use std::marker::PhantomData;

use precompile::Precompile;
use revm::{
    evm_impl::Transact,
    precompile::{self, Precompiles},
    Database, EVMData, Inspector, JournaledState,
};

use revm_interpreter::{
    return_ok, CallInputs, Contract, CreateInputs, Gas, Host, InstructionResult, Interpreter,
    SelfDestructResult, CALL_STACK_LIMIT,
};
use revm_primitives::{
    hash_map::Entry, Bytecode, Bytes, EVMResult, Env, HashMap, Spec, SpecId, B160, B256,
    KECCAK_EMPTY, U256,
};

pub(crate) struct DummyHost<'a, GSPEC: Spec, DB: Database, const INSPECT: bool> {
    pub storage: HashMap<U256, U256>,
    data: EVMData<'a, DB>,
    inspector: &'a mut dyn Inspector<DB>,
    hardcode: Option<Vec<u8>>,
    _phantomdata: PhantomData<GSPEC>,
}

pub(crate) struct DummySpec;

impl Spec for DummySpec {
    const SPEC_ID: SpecId = SpecId::LATEST;
}

impl<'a, GSPEC: Spec, DB: Database, const INSPECT: bool> Transact<DB::Error>
    for DummyHost<'a, GSPEC, DB, INSPECT>
{
    fn transact(&mut self) -> EVMResult<DB::Error> {
        panic!("transact")
    }
}

impl<'a, GSPEC: Spec, DB: Database, const INSPECT: bool> DummyHost<'a, GSPEC, DB, INSPECT> {
    pub fn new(
        db: &'a mut DB,
        env: &'a mut Env,
        inspector: &'a mut dyn Inspector<DB>,
        hardcode: Option<Vec<u8>>,
        precompiles: Precompiles,
    ) -> Self {
        let journaled_state = if GSPEC::enabled(SpecId::SPURIOUS_DRAGON) {
            JournaledState::new(precompiles.len())
        } else {
            JournaledState::new_legacy(precompiles.len())
        };
        Self {
            data: EVMData {
                env,
                journaled_state,
                db,
                error: None,
                precompiles,
            },
            inspector,
            storage: HashMap::new(),
            hardcode,
            _phantomdata: PhantomData {},
        }
    }

    /// Main contract call of the EVM.
    fn call_inner(&mut self, inputs: &mut CallInputs) -> (InstructionResult, Gas, Bytes) {
        // Call the inspector
        if INSPECT {
            let (ret, gas, out) = self
                .inspector
                .call(&mut self.data, inputs, inputs.is_static);
            if ret != InstructionResult::Continue {
                return self.inspector.call_end(
                    &mut self.data,
                    inputs,
                    gas,
                    ret,
                    out,
                    inputs.is_static,
                );
            }
        }

        let mut gas: Gas = Gas::new(inputs.gas_limit);
        // Load account and get code. Account is now hot.
        let bytecode: Bytecode = if let Some((bytecode, _)) = self.code(inputs.contract) {
            bytecode
        } else {
            return (InstructionResult::FatalExternalError, gas, Bytes::new());
        };

        // Check depth
        if self.data.journaled_state.depth() > CALL_STACK_LIMIT {
            let (ret, gas, out) = (InstructionResult::CallTooDeep, gas, Bytes::new());
            if INSPECT {
                return self.inspector.call_end(
                    &mut self.data,
                    inputs,
                    gas,
                    ret,
                    out,
                    inputs.is_static,
                );
            } else {
                return (ret, gas, out);
            }
        }

        // Create subroutine checkpoint
        let checkpoint = self.data.journaled_state.checkpoint();

        // Touch address. For "EIP-158 State Clear", this will erase empty accounts.
        if inputs.transfer.value == U256::ZERO {
            self.load_account(inputs.context.address);
            self.data.journaled_state.touch(&inputs.context.address);
        }

        // Transfer value from caller to called account
        if let Err(e) = self.data.journaled_state.transfer(
            &inputs.transfer.source,
            &inputs.transfer.target,
            inputs.transfer.value,
            self.data.db,
        ) {
            self.data.journaled_state.checkpoint_revert(checkpoint);
            let (ret, gas, out) = (e, gas, Bytes::new());
            if INSPECT {
                return self.inspector.call_end(
                    &mut self.data,
                    inputs,
                    gas,
                    ret,
                    out,
                    inputs.is_static,
                );
            } else {
                return (ret, gas, out);
            }
        }

        // Call precompiles
        let (ret, gas, out) = if let Some(precompile) = self.data.precompiles.get(&inputs.contract)
        {
            let out = match precompile {
                Precompile::Standard(fun) => fun(inputs.input.as_ref(), inputs.gas_limit),
                Precompile::Custom(fun) => fun(inputs.input.as_ref(), inputs.gas_limit),
            };
            match out {
                Ok((gas_used, data)) => {
                    if !revm::USE_GAS || gas.record_cost(gas_used) {
                        self.data.journaled_state.checkpoint_commit();
                        (InstructionResult::Return, gas, Bytes::from(data))
                    } else {
                        self.data.journaled_state.checkpoint_revert(checkpoint);
                        (InstructionResult::PrecompileOOG, gas, Bytes::new())
                    }
                }
                Err(e) => {
                    let ret = if let precompile::Error::OutOfGas = e {
                        InstructionResult::PrecompileOOG
                    } else {
                        InstructionResult::PrecompileError
                    };
                    self.data.journaled_state.checkpoint_revert(checkpoint);
                    (ret, gas, Bytes::new())
                }
            }
        } else {
            println!("#### calldata:{:?},", hex::encode(inputs.input.to_vec()));
            println!(
                "### bytecode:{:?},",
                hex::encode(bytecode.bytecode.to_vec())
            );
            // Create interpreter and execute subcall
            let contract =
                Contract::new_with_context(inputs.input.clone(), bytecode, &inputs.context);

            #[cfg(feature = "memory_limit")]
            let mut interpreter = Interpreter::new_with_memory_limit(
                contract,
                gas.limit(),
                inputs.is_static,
                self.data.env.cfg.memory_limit,
            );

            #[cfg(not(feature = "memory_limit"))]
            let mut interpreter = Interpreter::new(contract, gas.limit(), inputs.is_static);

            if INSPECT {
                // create is always no static call.
                self.inspector
                    .initialize_interp(&mut interpreter, &mut self.data, false);
            }
            let exit_reason = if INSPECT {
                interpreter.run_inspect::<Self, GSPEC>(self)
            } else {
                interpreter.run::<Self, GSPEC>(self)
            };

            if matches!(exit_reason, return_ok!()) {
                self.data.journaled_state.checkpoint_commit();
            } else {
                self.data.journaled_state.checkpoint_revert(checkpoint);
            }

            (exit_reason, interpreter.gas, interpreter.return_value())
        };

        if INSPECT {
            self.inspector
                .call_end(&mut self.data, inputs, gas, ret, out, inputs.is_static)
        } else {
            (ret, gas, out)
        }
    }
}

impl<'a, GSPEC: Spec, DB: Database, const INSPECT: bool> Host
    for DummyHost<'a, GSPEC, DB, INSPECT>
{
    fn step(&mut self, _interp: &mut Interpreter, _is_static: bool) -> InstructionResult {
        InstructionResult::Continue
    }

    fn step_end(
        &mut self,
        _interp: &mut Interpreter,
        _is_static: bool,
        _ret: InstructionResult,
    ) -> InstructionResult {
        InstructionResult::Continue
    }

    fn env(&mut self) -> &mut Env {
        &mut self.data.env
    }

    fn load_account(&mut self, _address: B160) -> Option<(bool, bool)> {
        Some((true, true))
    }

    fn block_hash(&mut self, _number: U256) -> Option<B256> {
        Some(B256::zero())
    }

    fn balance(&mut self, _address: B160) -> Option<(U256, bool)> {
        Some((U256::ZERO, false))
    }

    fn code(&mut self, address: B160) -> Option<(Bytecode, bool)> {
        if self.hardcode.is_some() {
            return Some((
                Bytecode::new_raw(Bytes::from(self.hardcode.clone().unwrap())),
                false,
            ));
        }

        let journal = &mut self.data.journaled_state;
        let db = &mut self.data.db;
        let error = &mut self.data.error;

        let (acc, is_cold) = journal
            .load_code(address, db)
            .map_err(|e| *error = Some(e))
            .ok()?;
        Some((acc.info.code.clone().unwrap(), is_cold))
    }

    fn code_hash(&mut self, __address: B160) -> Option<(B256, bool)> {
        Some((KECCAK_EMPTY, false))
    }

    fn sload(&mut self, __address: B160, index: U256) -> Option<(U256, bool)> {
        match self.storage.entry(index) {
            Entry::Occupied(entry) => Some((*entry.get(), false)),
            Entry::Vacant(entry) => {
                entry.insert(U256::ZERO);
                Some((U256::ZERO, true))
            }
        }
    }

    fn sstore(
        &mut self,
        _address: B160,
        index: U256,
        value: U256,
    ) -> Option<(U256, U256, U256, bool)> {
        let (present, is_cold) = match self.storage.entry(index) {
            Entry::Occupied(mut entry) => (entry.insert(value), false),
            Entry::Vacant(entry) => {
                entry.insert(value);
                (U256::ZERO, true)
            }
        };

        Some((U256::ZERO, present, value, is_cold))
    }

    fn log(&mut self, _address: B160, _topics: Vec<B256>, _data: Bytes) {
        println!("log")
    }

    fn selfdestruct(&mut self, _address: B160, _target: B160) -> Option<SelfDestructResult> {
        panic!("Selfdestruct is not supported for this host")
    }

    fn create(
        &mut self,
        _inputs: &mut CreateInputs,
    ) -> (InstructionResult, Option<B160>, Gas, Bytes) {
        (
            InstructionResult::Continue,
            None,
            Gas::new(0),
            Bytes::default(),
        )
    }

    fn call(&mut self, inputs: &mut CallInputs) -> (InstructionResult, Gas, Bytes) {
        self.call_inner(inputs)
    }
}
