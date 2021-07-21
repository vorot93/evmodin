use crate::{
    host::{AccessStatus, StorageStatus, TxContext},
    state::ExecutionState,
    *,
};
use arrayvec::ArrayVec;
use enum_as_inner::EnumAsInner;
use ethereum_types::*;

#[derive(Debug)]
pub enum Interrupt {
    InstructionStart {
        pc: usize,
        opcode: OpCode,
        state: ExecutionState,
    },
    AccountExists {
        address: Address,
    },
    GetStorage {
        address: Address,
        key: H256,
    },
    SetStorage {
        address: Address,
        key: H256,
        value: H256,
    },
    GetBalance {
        address: Address,
    },
    GetCodeSize {
        address: Address,
    },
    GetCodeHash {
        address: Address,
    },
    CopyCode {
        address: Address,
        offset: usize,
        max_size: usize,
    },
    Selfdestruct {
        address: Address,
        beneficiary: Address,
    },
    Call {
        message: Message,
    },
    GetTxContext,
    GetBlockHash {
        block_number: u64,
    },
    EmitLog {
        address: Address,
        data: Bytes,
        topics: ArrayVec<H256, 4>,
    },
    AccessAccount {
        address: Address,
    },
    AccessStorage {
        address: Address,
        key: H256,
    },
}

#[derive(Debug, EnumAsInner)]
pub enum ResumeData {
    Empty,
    AccountExists { exists: bool },
    Balance { balance: U256 },
    CodeSize { code_size: U256 },
    StorageValue { value: H256 },
    StorageStatus { status: StorageStatus },
    CodeHash { hash: H256 },
    BlockHash { hash: H256 },
    TxContext { context: TxContext },
    Code { code: Bytes },
    CallOutput { output: Output },
    AccessAccount { status: AccessStatus },
    AccessStorage { status: AccessStatus },
}
