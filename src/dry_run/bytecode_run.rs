use crate::dry_run::dummy::*;
use crate::dry_run::error::{Error, Result};

use revm::inspectors::NoOpInspector;
use revm::InMemoryDB;
use revm_interpreter::{return_ok, CallContext, Contract, InstructionResult, Interpreter};
use revm_precompile::Precompiles;
use revm_primitives::{Bytecode, BytecodeState, Env};

use super::dummy;

pub fn bytecode_run(
    calldata: Vec<u8>,
    bytecode: Vec<u8>,
    hardcode: Option<Vec<u8>>,
) -> Result<Vec<u8>> {
    let call_context = CallContext::default();
    let bytecode = Bytecode {
        bytecode: bytecode.into(),
        state: BytecodeState::Raw,
        ..Default::default()
    };

    let contract = Contract::new_with_context(calldata.into(), bytecode, &call_context);
    let mut interpreter = Interpreter::new(contract, u64::MAX, false);

    let mut noop = NoOpInspector {};
    let mut db = InMemoryDB::default();
    let mut env = Env::default();
    let mut host: dummy::DummyHost<'_, DummySpec, _, false> = dummy::DummyHost::new(
        &mut db,
        &mut env,
        &mut noop,
        hardcode,
        Precompiles::new(revm_precompile::SpecId::LATEST).clone(),
    );
    let result = interpreter.run::<_, DummySpec>(&mut host);

    if matches!(result, return_ok!()) {
        Ok(interpreter.return_value().to_vec())
    } else {
        Err(Error::InterpreterError(format!("{result:?}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // Test with compiled bytecode of https://gitlab.com/dompute/decompile-test/-/blame/main/src/BasicToken.sol#L28
        let bytecode = hex::decode("608060405234801561001057600080fd5b506004361061004c5760003560e01c806318160ddd1461005157806370a0823114610068578063771602f714610091578063a9059cbb146100a4575b600080fd5b6000545b6040519081526020015b60405180910390f35b610055610076366004610198565b6001600160a01b031660009081526001602052604090205490565b61005561009f3660046101b3565b6100c7565b6100b76100b23660046101d5565b6100dc565b604051901515815260200161005f565b60006100d38284610215565b90505b92915050565b60006001600160a01b0383166100f157600080fd5b3360009081526001602052604090205482111561010d57600080fd5b33600090815260016020526040902054610128908390610228565b33600090815260016020526040808220929092556001600160a01b03851681522054610155908390610215565b6001600160a01b038416600090815260016020819052604090912091909155905092915050565b80356001600160a01b038116811461019357600080fd5b919050565b6000602082840312156101aa57600080fd5b6100d38261017c565b600080604083850312156101c657600080fd5b50508035926020909101359150565b600080604083850312156101e857600080fd5b6101f18361017c565b946020939093013593505050565b634e487b7160e01b600052601160045260246000fd5b808201808211156100d6576100d66101ff565b818103818111156100d6576100d66101ff56fea26469706673582212203c8cf1d0b0ffb741e4b0758b951e25d3fde6108d8823a4ae95a0c0fe926284bf64736f6c63430008150033").unwrap();
        // call pure function `add(uint256, uint256)` with params 2 and 3
        let calldata = hex::decode("771602f700000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000003").unwrap();

        let result = bytecode_run(calldata, bytecode, None);
        assert!(result.is_ok());
        // 2 + 3 = 5
        assert_eq!(
            result.unwrap(),
            hex::decode("0000000000000000000000000000000000000000000000000000000000000005")
                .unwrap()
        );
    }
}
