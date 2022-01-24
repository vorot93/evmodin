use self::{interrupt::*, interrupt_data::*, resume_data::*};
use crate::{
    common::*,
    host::{AccessStatus, StorageStatus, TxContext},
    state::ExecutionState,
    *,
};
use arrayvec::ArrayVec;
use derive_more::From;
use enum_as_inner::EnumAsInner;
use ethereum_types::Address;
use ethnum::U256;
use std::ops::GeneratorState;
use std::{convert::Infallible, ops::Generator, pin::Pin};

mod sealed {
    pub trait Sealed {}
}

/// Interrupts.
pub mod interrupt;
/// Data attached to interrupts.
pub mod interrupt_data;
/// Data required for resume.
pub mod resume_data;

/// Paused EVM with full state inside.
pub trait Interrupt: sealed::Sealed {
    /// Data required to resume execution.
    type ResumeData;

    /// Resume execution until the next interrupt.
    fn resume(self, resume_data: Self::ResumeData) -> InterruptVariant;
}

pub(crate) type InnerCoroutine = Box<
    dyn Generator<
            ResumeDataVariant,
            Yield = InterruptDataVariant,
            Return = Result<SuccessfulOutput, StatusCode>,
        > + Send
        + Sync
        + Unpin,
>;

fn resume_interrupt(mut inner: InnerCoroutine, resume_data: ResumeDataVariant) -> InterruptVariant {
    match Pin::new(&mut *inner).resume(resume_data) {
        GeneratorState::Yielded(interrupt) => match interrupt {
            InterruptDataVariant::InstructionStart(data) => {
                InterruptVariant::InstructionStart(data, InstructionStartInterrupt { inner })
            }
            InterruptDataVariant::AccountExists(data) => {
                InterruptVariant::AccountExists(data, AccountExistsInterrupt { inner })
            }
            InterruptDataVariant::GetStorage(data) => {
                InterruptVariant::GetStorage(data, GetStorageInterrupt { inner })
            }
            InterruptDataVariant::SetStorage(data) => {
                InterruptVariant::SetStorage(data, SetStorageInterrupt { inner })
            }
            InterruptDataVariant::GetBalance(data) => {
                InterruptVariant::GetBalance(data, GetBalanceInterrupt { inner })
            }
            InterruptDataVariant::GetCodeSize(data) => {
                InterruptVariant::GetCodeSize(data, GetCodeSizeInterrupt { inner })
            }
            InterruptDataVariant::GetCodeHash(data) => {
                InterruptVariant::GetCodeHash(data, GetCodeHashInterrupt { inner })
            }
            InterruptDataVariant::CopyCode(data) => {
                InterruptVariant::CopyCode(data, CopyCodeInterrupt { inner })
            }
            InterruptDataVariant::Selfdestruct(data) => {
                InterruptVariant::Selfdestruct(data, SelfdestructInterrupt { inner })
            }
            InterruptDataVariant::Call(data) => {
                InterruptVariant::Call(data, CallInterrupt { inner })
            }
            InterruptDataVariant::GetTxContext => {
                InterruptVariant::GetTxContext(GetTxContextInterrupt { inner })
            }
            InterruptDataVariant::GetBlockHash(data) => {
                InterruptVariant::GetBlockHash(data, GetBlockHashInterrupt { inner })
            }
            InterruptDataVariant::EmitLog(data) => {
                InterruptVariant::EmitLog(data, EmitLogInterrupt { inner })
            }
            InterruptDataVariant::AccessAccount(data) => {
                InterruptVariant::AccessAccount(data, AccessAccountInterrupt { inner })
            }
            InterruptDataVariant::AccessStorage(data) => {
                InterruptVariant::AccessStorage(data, AccessStorageInterrupt { inner })
            }
        },
        GeneratorState::Complete(res) => InterruptVariant::Complete(res, ExecutionComplete(inner)),
    }
}
