use crate::{
    common::*,
    host::{AccessStatus, StorageStatus, TxContext},
    state::ExecutionState,
    *,
};
use arrayvec::ArrayVec;
use derive_more::From;
use enum_as_inner::EnumAsInner;
use ethereum_types::*;
use genawaiter::{Coroutine, GeneratorState};
use std::{convert::Infallible, pin::Pin};

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
    pub key: H256,
}

#[derive(Debug)]
pub struct SetStorage {
    pub address: Address,
    pub key: H256,
    pub value: H256,
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
pub struct Call {
    pub message: Message,
}

#[derive(Debug)]
pub struct GetBlockHash {
    pub block_number: u64,
}

#[derive(Debug)]
pub struct EmitLog {
    pub address: Address,
    pub data: Bytes,
    pub topics: ArrayVec<H256, 4>,
}

#[derive(Debug)]
pub struct AccessAccount {
    pub address: Address,
}

#[derive(Debug)]
pub struct AccessStorage {
    pub address: Address,
    pub key: H256,
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

#[derive(Debug)]
pub struct AccountExistsStatus {
    pub exists: bool,
}

#[derive(Debug)]
pub struct Balance {
    pub balance: U256,
}

#[derive(Debug)]
pub struct CodeSize {
    pub code_size: U256,
}

#[derive(Debug)]
pub struct StorageValue {
    pub value: H256,
}

#[derive(Debug)]
pub struct StorageStatusInfo {
    pub status: StorageStatus,
}

#[derive(Debug)]
pub struct CodeHash {
    pub hash: H256,
}

#[derive(Debug)]
pub struct BlockHash {
    pub hash: H256,
}

#[derive(Debug)]
pub struct TxContextData {
    pub context: TxContext,
}

#[derive(Debug)]
pub struct Code {
    pub code: Bytes,
}

#[derive(Debug)]
pub struct CallOutput {
    pub output: Output,
}

#[derive(Debug)]
pub struct AccessAccountStatus {
    pub status: AccessStatus,
}

#[derive(Debug)]
pub struct AccessStorageStatus {
    pub status: AccessStatus,
}

#[derive(Debug, EnumAsInner, From)]
pub enum ResumeDataVariant {
    #[from(ignore)]
    Empty,
    AccountExistsStatus(AccountExistsStatus),
    Balance(Balance),
    CodeSize(CodeSize),
    StorageValue(StorageValue),
    StorageStatusInfo(StorageStatusInfo),
    CodeHash(CodeHash),
    BlockHash(BlockHash),
    TxContextData(TxContextData),
    Code(Code),
    CallOutput(CallOutput),
    AccessAccountStatus(AccessAccountStatus),
    AccessStorageStatus(AccessStorageStatus),
    Done(Infallible),
}

impl From<()> for ResumeDataVariant {
    fn from(_: ()) -> Self {
        Self::Empty
    }
}

pub trait Interrupt {
    type ResumeData;

    fn resume(self, resume_data: Self::ResumeData) -> InterruptVariant;
}

macro_rules! interrupt {
    ($name:ident, $data:ty, $resume_with:ty) => {
        pub struct $name {
            pub(crate) inner: ::core::pin::Pin<
                Box<
                    dyn Coroutine<
                            Yield = InterruptDataVariant,
                            Resume = ResumeDataVariant,
                            Return = Result<SuccessfulOutput, StatusCode>,
                        > + Send
                        + Sync
                        + Unpin,
                >,
            >,
            pub data: $data,
        }

        impl Interrupt for $name {
            type ResumeData = $resume_with;

            fn resume(self, resume_data: $resume_with) -> InterruptVariant {
                resume_interrupt(self.inner, resume_data.into())
            }
        }
    };
}

interrupt!(ExecutionStartInterrupt, (), ());
interrupt!(InstructionStartInterrupt, Box<InstructionStart>, ());
interrupt!(AccountExistsInterrupt, AccountExists, AccountExistsStatus);
interrupt!(GetStorageInterrupt, GetStorage, StorageValue);
interrupt!(SetStorageInterrupt, SetStorage, StorageStatusInfo);
interrupt!(GetBalanceInterrupt, GetBalance, Balance);
interrupt!(GetCodeSizeInterrupt, GetCodeSize, CodeSize);
interrupt!(GetCodeHashInterrupt, GetCodeHash, CodeHash);
interrupt!(CopyCodeInterrupt, CopyCode, Code);
interrupt!(SelfdestructInterrupt, Selfdestruct, ());
interrupt!(CallInterrupt, Call, CallOutput);
interrupt!(GetTxContextInterrupt, (), TxContextData);
interrupt!(GetBlockHashInterrupt, GetBlockHash, BlockHash);
interrupt!(EmitLogInterrupt, EmitLog, ());
interrupt!(AccessAccountInterrupt, AccessAccount, AccessAccountStatus);
interrupt!(AccessStorageInterrupt, AccessStorage, AccessStorageStatus);
interrupt!(CompleteInterrupt, Result<SuccessfulOutput, StatusCode>, Infallible);

#[derive(From)]
pub enum InterruptVariant {
    ExecutionStart(ExecutionStartInterrupt),
    InstructionStart(InstructionStartInterrupt),
    AccountExists(AccountExistsInterrupt),
    GetStorage(GetStorageInterrupt),
    SetStorage(SetStorageInterrupt),
    GetBalance(GetBalanceInterrupt),
    GetCodeSize(GetCodeSizeInterrupt),
    GetCodeHash(GetCodeHashInterrupt),
    CopyCode(CopyCodeInterrupt),
    Selfdestruct(SelfdestructInterrupt),
    Call(CallInterrupt),
    GetTxContext(GetTxContextInterrupt),
    GetBlockHash(GetBlockHashInterrupt),
    EmitLog(EmitLogInterrupt),
    AccessAccount(AccessAccountInterrupt),
    AccessStorage(AccessStorageInterrupt),
    Complete(CompleteInterrupt),
}

type InnerCoroutine = Pin<
    Box<
        dyn Coroutine<
                Yield = InterruptDataVariant,
                Resume = ResumeDataVariant,
                Return = Result<SuccessfulOutput, StatusCode>,
            > + Send
            + Sync
            + Unpin,
    >,
>;

fn resume_interrupt(mut inner: InnerCoroutine, resume_data: ResumeDataVariant) -> InterruptVariant {
    match Pin::new(&mut *inner).resume_with(resume_data) {
        GeneratorState::Yielded(interrupt) => match interrupt {
            InterruptDataVariant::InstructionStart(data) => {
                InstructionStartInterrupt { inner, data }.into()
            }
            InterruptDataVariant::AccountExists(data) => {
                AccountExistsInterrupt { inner, data }.into()
            }
            InterruptDataVariant::GetStorage(data) => GetStorageInterrupt { inner, data }.into(),
            InterruptDataVariant::SetStorage(data) => SetStorageInterrupt { inner, data }.into(),
            InterruptDataVariant::GetBalance(data) => GetBalanceInterrupt { inner, data }.into(),
            InterruptDataVariant::GetCodeSize(data) => GetCodeSizeInterrupt { inner, data }.into(),
            InterruptDataVariant::GetCodeHash(data) => GetCodeHashInterrupt { inner, data }.into(),
            InterruptDataVariant::CopyCode(data) => CopyCodeInterrupt { inner, data }.into(),
            InterruptDataVariant::Selfdestruct(data) => {
                SelfdestructInterrupt { inner, data }.into()
            }
            InterruptDataVariant::Call(data) => CallInterrupt { inner, data }.into(),
            InterruptDataVariant::GetTxContext => GetTxContextInterrupt { inner, data: () }.into(),
            InterruptDataVariant::GetBlockHash(data) => {
                GetBlockHashInterrupt { inner, data }.into()
            }
            InterruptDataVariant::EmitLog(data) => EmitLogInterrupt { inner, data }.into(),
            InterruptDataVariant::AccessAccount(data) => {
                AccessAccountInterrupt { inner, data }.into()
            }
            InterruptDataVariant::AccessStorage(data) => {
                AccessStorageInterrupt { inner, data }.into()
            }
        },
        GeneratorState::Complete(data) => {
            InterruptVariant::Complete(CompleteInterrupt { inner, data })
        }
    }
}
