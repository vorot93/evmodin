use super::*;

macro_rules! interrupt {
    ( $(#[$outer:meta])* $name:ident, $data:ty => $resume_with:ty) => {
		$(#[$outer])*
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
            /// Data returned by this interrupt.
            pub data: $data,
        }

        impl sealed::Sealed for $name {}

        impl Interrupt for $name {
            type ResumeData = $resume_with;

            fn resume(self, resume_data: $resume_with) -> InterruptVariant {
                resume_interrupt(self.inner, resume_data.into())
            }
        }
    };
}

interrupt! {
    /// EVM has just been created. Resume this interrupt to start execution.
    ExecutionStartInterrupt,
    () => ()
}
interrupt! {
    /// New instruction has been encountered.
    InstructionStartInterrupt,
    Box<InstructionStart> => ()
}
interrupt! {
    /// Does this account exist?
    AccountExistsInterrupt,
    AccountExists => AccountExistsStatus
}
interrupt! {
    /// Need this storage key.
    GetStorageInterrupt,
    GetStorage => StorageValue
}
interrupt! {
    /// Set this storage key.
    SetStorageInterrupt,
    SetStorage => StorageStatusInfo
}
interrupt! {
    /// Get balance of this account.
    GetBalanceInterrupt,
    GetBalance => Balance
}
interrupt! {
    /// Get code size of this account.
    GetCodeSizeInterrupt,
    GetCodeSize => CodeSize
}
interrupt! {
    /// Get code hash of this account.
    GetCodeHashInterrupt,
    GetCodeHash => CodeHash
}
interrupt! {
    /// Get code of this account.
    CopyCodeInterrupt,
    CopyCode => Code
}
interrupt! {
    /// Selfdestruct this account.
    SelfdestructInterrupt,
    Selfdestruct => ()
}
interrupt! {
    /// Execute this message as a new call.
    CallInterrupt,
    Call => CallOutput
}
interrupt! {
    /// Get `TxContext` for this call.
    GetTxContextInterrupt,
    () => TxContextData
}
interrupt! {
    /// Get block hash for this account.
    GetBlockHashInterrupt,
    GetBlockHash => BlockHash
}
interrupt! {
    /// Emit log message.
    EmitLogInterrupt,
    EmitLog => ()
}
interrupt! {
    /// Access this account and return its status.
    AccessAccountInterrupt,
    AccessAccount => AccessAccountStatus
}
interrupt! {
    /// Access this storage key and return its status.
    AccessStorageInterrupt,
    AccessStorage => AccessStorageStatus
}
interrupt! {
    /// Execution complete. Output is attached.
    ///
    /// NOTE: this is a special interrupt. It cannot be resumed.
    CompleteInterrupt,
    Result<SuccessfulOutput, StatusCode> => Infallible
}

/// Collection of all possible interrupts. Match on this to get the specific interrupt returned.
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
