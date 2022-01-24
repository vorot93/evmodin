use super::*;

macro_rules! interrupt {
    ( $(#[$outer:meta])* $name:ident => $resume_with:ty) => {
		$(#[$outer])*
        pub struct $name {
            pub(crate) inner: InnerCoroutine,
        }

        impl sealed::Sealed for $name {}

        impl Interrupt for $name {
            type ResumeData = $resume_with;

            fn resume(self, resume_data: Self::ResumeData) -> InterruptVariant {
                resume_interrupt(self.inner, resume_data.into())
            }
        }
    };
}

interrupt! {
    /// EVM has just been created. Resume this interrupt to start execution.
    ExecutionStartInterrupt => ()
}
interrupt! {
    /// New instruction has been encountered.
    InstructionStartInterrupt => StateModifier
}
interrupt! {
    /// Does this account exist?
    AccountExistsInterrupt => AccountExistsStatus
}
interrupt! {
    /// Need this storage key.
    GetStorageInterrupt => StorageValue
}
interrupt! {
    /// Set this storage key.
    SetStorageInterrupt => StorageStatusInfo
}
interrupt! {
    /// Get balance of this account.
    GetBalanceInterrupt => Balance
}
interrupt! {
    /// Get code size of this account.
    GetCodeSizeInterrupt => CodeSize
}
interrupt! {
    /// Get code hash of this account.
    GetCodeHashInterrupt => CodeHash
}
interrupt! {
    /// Get code of this account.
    CopyCodeInterrupt  => Code
}
interrupt! {
    /// Selfdestruct this account.
    SelfdestructInterrupt  => ()
}
interrupt! {
    /// Execute this message as a new call.
    CallInterrupt => CallOutput
}
interrupt! {
    /// Get `TxContext` for this call.
    GetTxContextInterrupt => TxContextData
}
interrupt! {
    /// Get block hash for this account.
    GetBlockHashInterrupt => BlockHash
}
interrupt! {
    /// Emit log message.
    EmitLogInterrupt => ()
}
interrupt! {
    /// Access this account and return its status.
    AccessAccountInterrupt => AccessAccountStatus
}
interrupt! {
    /// Access this storage key and return its status.
    AccessStorageInterrupt => AccessStorageStatus
}

/// Execution complete, this interrupt cannot be resumed.
pub struct ExecutionComplete(pub(crate) InnerCoroutine);

/// Collection of all possible interrupts. Match on this to get the specific interrupt returned.
#[derive(From)]
pub enum InterruptVariant {
    InstructionStart(Box<InstructionStart>, InstructionStartInterrupt),
    AccountExists(AccountExists, AccountExistsInterrupt),
    GetStorage(GetStorage, GetStorageInterrupt),
    SetStorage(SetStorage, SetStorageInterrupt),
    GetBalance(GetBalance, GetBalanceInterrupt),
    GetCodeSize(GetCodeSize, GetCodeSizeInterrupt),
    GetCodeHash(GetCodeHash, GetCodeHashInterrupt),
    CopyCode(CopyCode, CopyCodeInterrupt),
    Selfdestruct(Selfdestruct, SelfdestructInterrupt),
    Call(Call, CallInterrupt),
    GetTxContext(GetTxContextInterrupt),
    GetBlockHash(GetBlockHash, GetBlockHashInterrupt),
    EmitLog(EmitLog, EmitLogInterrupt),
    AccessAccount(AccessAccount, AccessAccountInterrupt),
    AccessStorage(AccessStorage, AccessStorageInterrupt),
    Complete(Result<SuccessfulOutput, StatusCode>, ExecutionComplete),
}
