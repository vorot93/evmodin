use crate::{host::AccessStatus, state::ExecutionState, *};
use ethereum_types::*;

pub enum Interrupt {
    InstructionStart {
        pc: usize,
        opcode: OpCode,
        state: ExecutionState,
    },
    AccessAccount {
        address: Address,
    },
    GetBalance {
        address: Address,
    },
}

pub enum ResumeData {
    Dummy,
    AccessAccount { status: AccessStatus },
    GetBalance { balance: U256 },
}
