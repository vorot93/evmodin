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
use ethereum_types::*;
use genawaiter::{Coroutine, GeneratorState};
use std::{convert::Infallible, pin::Pin};

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
    /// Interrupt data returned.
    type InterruptData;
    /// Data required to resume execution.
    type ResumeData;

    /// Get interrupt data.
    fn data(&self) -> &Self::InterruptData;
    /// Resume execution until the next interrupt.
    fn resume(self, resume_data: Self::ResumeData) -> InterruptVariant;
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
        GeneratorState::Complete(res) => InterruptVariant::Complete(res),
    }
}
