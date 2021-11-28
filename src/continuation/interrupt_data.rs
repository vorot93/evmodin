use super::*;

#[derive(Debug)]
pub struct InstructionStart {
    pub pc: usize,
    pub opcode: OpCode,
    pub state: ExecutionState,
}

#[derive(Debug)]
pub struct AccountExists {
    pub address: Address,
}

#[derive(Debug)]
pub struct GetStorage {
    pub address: Address,
    pub key: U256,
}

#[derive(Debug)]
pub struct SetStorage {
    pub address: Address,
    pub key: U256,
    pub value: U256,
}

#[derive(Debug)]
pub struct GetBalance {
    pub address: Address,
}

#[derive(Debug)]
pub struct GetCodeSize {
    pub address: Address,
}

#[derive(Debug)]
pub struct GetCodeHash {
    pub address: Address,
}

#[derive(Debug)]
pub struct CopyCode {
    pub address: Address,
    pub offset: usize,
    pub max_size: usize,
}

#[derive(Debug)]
pub struct Selfdestruct {
    pub address: Address,
    pub beneficiary: Address,
}

#[derive(Debug)]
pub enum Call {
    Call(Message),
    Create(CreateMessage),
}

#[derive(Debug)]
pub struct GetBlockHash {
    pub block_number: u64,
}

#[derive(Debug)]
pub struct EmitLog {
    pub address: Address,
    pub data: Bytes,
    pub topics: ArrayVec<U256, 4>,
}

#[derive(Debug)]
pub struct AccessAccount {
    pub address: Address,
}

#[derive(Debug)]
pub struct AccessStorage {
    pub address: Address,
    pub key: U256,
}

#[derive(Debug)]
pub enum InterruptDataVariant {
    InstructionStart(Box<InstructionStart>),
    AccountExists(AccountExists),
    GetStorage(GetStorage),
    SetStorage(SetStorage),
    GetBalance(GetBalance),
    GetCodeSize(GetCodeSize),
    GetCodeHash(GetCodeHash),
    CopyCode(CopyCode),
    Selfdestruct(Selfdestruct),
    Call(Call),
    GetTxContext,
    GetBlockHash(GetBlockHash),
    EmitLog(EmitLog),
    AccessAccount(AccessAccount),
    AccessStorage(AccessStorage),
}
