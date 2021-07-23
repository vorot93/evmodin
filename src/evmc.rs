use crate::{common::*, host::*, tracing::NoopTracer, AnalyzedCode};
use ::evmc_vm;
use ::evmc_vm::{ffi::*, EvmcVm, ExecutionContext, ExecutionMessage, MessageFlags, MessageKind};
use arrayvec::ArrayVec;
use bytes::Bytes;
use ethereum_types::{Address, H256, U256};
use evmc_vm::ExecutionResult;
use std::convert::TryInto;

pub(crate) trait Convert {
    type Into;

    fn convert(self) -> Self::Into;
}

impl Convert for Address {
    type Into = evmc_address;

    fn convert(self) -> Self::Into {
        evmc_address {
            bytes: self.to_fixed_bytes(),
        }
    }
}

impl Convert for H256 {
    type Into = evmc_bytes32;

    fn convert(self) -> Self::Into {
        evmc_bytes32 {
            bytes: self.to_fixed_bytes(),
        }
    }
}

impl Convert for U256 {
    type Into = evmc_uint256be;

    fn convert(self) -> Self::Into {
        evmc_uint256be { bytes: self.into() }
    }
}

impl From<evmc_access_status> for AccessStatus {
    fn from(s: evmc_access_status) -> Self {
        match s {
            evmc_access_status::EVMC_ACCESS_COLD => Self::Cold,
            evmc_access_status::EVMC_ACCESS_WARM => Self::Warm,
        }
    }
}

impl From<Message> for ExecutionMessage {
    fn from(msg: Message) -> Self {
        let mut create2_salt = evmc_bytes32::default();
        let kind = match msg.kind {
            crate::CallKind::Call => MessageKind::EVMC_CALL,
            crate::CallKind::DelegateCall => MessageKind::EVMC_DELEGATECALL,
            crate::CallKind::CallCode => MessageKind::EVMC_CALLCODE,
            crate::CallKind::Create => MessageKind::EVMC_CREATE,
            crate::CallKind::Create2 { salt } => {
                create2_salt = salt.convert();
                MessageKind::EVMC_CREATE2
            }
        };
        let flags = if msg.is_static {
            MessageFlags::EVMC_STATIC as u32
        } else {
            0
        };

        ExecutionMessage::new(
            kind,
            flags,
            msg.depth,
            msg.gas,
            msg.destination.convert(),
            msg.sender.convert(),
            msg.input_data.is_empty().then(|| &*msg.input_data),
            msg.value.convert(),
            create2_salt,
        )
    }
}

impl Message {
    fn from_evmc(msg: &ExecutionMessage) -> Self {
        let kind = match msg.kind() {
            evmc_call_kind::EVMC_CALL => CallKind::Call,
            evmc_call_kind::EVMC_DELEGATECALL => CallKind::DelegateCall,
            evmc_call_kind::EVMC_CALLCODE => CallKind::CallCode,
            evmc_call_kind::EVMC_CREATE => CallKind::Create,
            evmc_call_kind::EVMC_CREATE2 => CallKind::Create2 {
                salt: msg.create2_salt().bytes.into(),
            },
        };

        Self {
            kind,
            is_static: msg.flags() == evmc_vm::ffi::evmc_flags::EVMC_STATIC as u32,
            depth: msg.depth(),
            gas: msg.gas(),
            destination: msg.destination().bytes.into(),
            sender: msg.sender().bytes.into(),
            input_data: msg
                .input()
                .map(|v| v.clone().into())
                .unwrap_or_else(Bytes::new),
            value: msg.value().bytes.into(),
        }
    }
}

impl From<evmc_status_code> for StatusCode {
    fn from(status: evmc_status_code) -> Self {
        match status {
            evmc_status_code::EVMC_SUCCESS => StatusCode::Success,
            evmc_status_code::EVMC_FAILURE => StatusCode::Failure,
            evmc_status_code::EVMC_REVERT => StatusCode::Revert,
            evmc_status_code::EVMC_OUT_OF_GAS => StatusCode::OutOfGas,
            evmc_status_code::EVMC_INVALID_INSTRUCTION => StatusCode::InvalidInstruction,
            evmc_status_code::EVMC_UNDEFINED_INSTRUCTION => StatusCode::UndefinedInstruction,
            evmc_status_code::EVMC_STACK_OVERFLOW => StatusCode::StackOverflow,
            evmc_status_code::EVMC_STACK_UNDERFLOW => StatusCode::StackUnderflow,
            evmc_status_code::EVMC_BAD_JUMP_DESTINATION => StatusCode::BadJumpDestination,
            evmc_status_code::EVMC_INVALID_MEMORY_ACCESS => StatusCode::InvalidMemoryAccess,
            evmc_status_code::EVMC_CALL_DEPTH_EXCEEDED => StatusCode::CallDepthExceeded,
            evmc_status_code::EVMC_STATIC_MODE_VIOLATION => StatusCode::StaticModeViolation,
            evmc_status_code::EVMC_PRECOMPILE_FAILURE => StatusCode::PrecompileFailure,
            evmc_status_code::EVMC_CONTRACT_VALIDATION_FAILURE => {
                StatusCode::InternalError("ContractValidationFailure".into())
            }
            evmc_status_code::EVMC_ARGUMENT_OUT_OF_RANGE => StatusCode::ArgumentOutOfRange,
            evmc_status_code::EVMC_WASM_UNREACHABLE_INSTRUCTION => {
                StatusCode::InternalError("WasmUnreachableInstruction".into())
            }
            evmc_status_code::EVMC_WASM_TRAP => StatusCode::InternalError("WasmTrap".into()),
            evmc_status_code::EVMC_INSUFFICIENT_BALANCE => StatusCode::InsufficientBalance,
            evmc_status_code::EVMC_INTERNAL_ERROR => StatusCode::InternalError(String::new()),
            evmc_status_code::EVMC_REJECTED => StatusCode::InternalError("Rejected".into()),
            evmc_status_code::EVMC_OUT_OF_MEMORY => StatusCode::InternalError("OutOfMemory".into()),
        }
    }
}

impl From<StatusCode> for evmc_status_code {
    fn from(status: StatusCode) -> Self {
        match status {
            StatusCode::Success => evmc_status_code::EVMC_SUCCESS,
            StatusCode::Failure => evmc_status_code::EVMC_FAILURE,
            StatusCode::Revert => evmc_status_code::EVMC_REVERT,
            StatusCode::OutOfGas => evmc_status_code::EVMC_OUT_OF_GAS,
            StatusCode::InvalidInstruction => evmc_status_code::EVMC_INVALID_INSTRUCTION,
            StatusCode::UndefinedInstruction => evmc_status_code::EVMC_UNDEFINED_INSTRUCTION,
            StatusCode::StackOverflow => evmc_status_code::EVMC_STACK_OVERFLOW,
            StatusCode::StackUnderflow => evmc_status_code::EVMC_STACK_UNDERFLOW,
            StatusCode::BadJumpDestination => evmc_status_code::EVMC_BAD_JUMP_DESTINATION,
            StatusCode::InvalidMemoryAccess => evmc_status_code::EVMC_INVALID_MEMORY_ACCESS,
            StatusCode::CallDepthExceeded => evmc_status_code::EVMC_CALL_DEPTH_EXCEEDED,
            StatusCode::StaticModeViolation => evmc_status_code::EVMC_STATIC_MODE_VIOLATION,
            StatusCode::PrecompileFailure => evmc_status_code::EVMC_PRECOMPILE_FAILURE,
            StatusCode::ArgumentOutOfRange => evmc_status_code::EVMC_ARGUMENT_OUT_OF_RANGE,
            StatusCode::InsufficientBalance => evmc_status_code::EVMC_INSUFFICIENT_BALANCE,
            StatusCode::InternalError(_) => evmc_status_code::EVMC_INTERNAL_ERROR,
        }
    }
}

impl<'a> Host for ExecutionContext<'a> {
    fn account_exists(&self, address: Address) -> bool {
        ExecutionContext::account_exists(self, &address.convert())
    }

    fn get_storage(&self, address: Address, key: H256) -> H256 {
        ExecutionContext::get_storage(self, &address.convert(), &key.convert())
            .bytes
            .into()
    }

    fn set_storage(&mut self, address: Address, key: H256, value: H256) -> StorageStatus {
        match ExecutionContext::set_storage(
            self,
            &address.convert(),
            &key.convert(),
            &value.convert(),
        ) {
            evmc_storage_status::EVMC_STORAGE_UNCHANGED => StorageStatus::Unchanged,
            evmc_storage_status::EVMC_STORAGE_MODIFIED => StorageStatus::Modified,
            evmc_storage_status::EVMC_STORAGE_MODIFIED_AGAIN => StorageStatus::ModifiedAgain,
            evmc_storage_status::EVMC_STORAGE_ADDED => StorageStatus::Added,
            evmc_storage_status::EVMC_STORAGE_DELETED => StorageStatus::Deleted,
        }
    }

    fn get_balance(&self, address: Address) -> U256 {
        ExecutionContext::get_balance(self, &address.convert())
            .bytes
            .into()
    }

    fn get_code_size(&self, address: Address) -> U256 {
        ExecutionContext::get_code_size(self, &address.convert()).into()
    }

    fn get_code_hash(&self, address: Address) -> H256 {
        ExecutionContext::get_code_hash(self, &address.convert())
            .bytes
            .into()
    }

    fn copy_code(&self, address: Address, offset: usize, buffer: &mut [u8]) -> usize {
        ExecutionContext::copy_code(self, &address.convert(), offset, buffer)
    }

    fn selfdestruct(&mut self, address: Address, beneficiary: Address) {
        ExecutionContext::selfdestruct(self, &address.convert(), &beneficiary.convert())
    }

    fn call(&mut self, msg: &Message) -> Output {
        let execution_result = ExecutionContext::call(self, &msg.clone().into());

        Output {
            status_code: execution_result.status_code().into(),
            gas_left: execution_result.gas_left(),
            output_data: execution_result
                .output()
                .map(|v| v.to_vec().into())
                .unwrap_or_else(Bytes::new),
            create_address: execution_result.create_address().map(|a| a.bytes.into()),
        }
    }

    fn get_tx_context(&self) -> TxContext {
        let c = ExecutionContext::get_tx_context(self);

        TxContext {
            tx_gas_price: c.tx_gas_price.bytes.into(),
            tx_origin: c.tx_origin.bytes.into(),
            block_coinbase: c.block_coinbase.bytes.into(),
            block_number: c.block_number.try_into().unwrap(),
            block_timestamp: c.block_timestamp.try_into().unwrap(),
            block_gas_limit: c.block_gas_limit.try_into().unwrap(),
            block_difficulty: c.block_difficulty.bytes.into(),
            chain_id: c.chain_id.bytes.into(),
            block_base_fee: c.block_base_fee.bytes.into(),
        }
    }

    fn get_block_hash(&self, block_number: u64) -> H256 {
        ExecutionContext::get_block_hash(self, block_number.try_into().unwrap())
            .bytes
            .into()
    }

    fn emit_log(&mut self, address: Address, data: &[u8], topics: &[H256]) {
        ExecutionContext::emit_log(
            self,
            &address.convert(),
            data,
            &topics
                .iter()
                .map(|topic| topic.convert())
                .collect::<ArrayVec<_, 4>>(),
        )
    }

    fn access_account(&mut self, address: Address) -> AccessStatus {
        ExecutionContext::access_account(self, &address.convert()).into()
    }

    fn access_storage(&mut self, address: Address, key: H256) -> AccessStatus {
        ExecutionContext::access_storage(self, &address.convert(), &key.convert()).into()
    }
}

impl From<evmc_vm::Revision> for Revision {
    fn from(rev: evmc_vm::Revision) -> Self {
        match rev {
            evmc_revision::EVMC_FRONTIER => Revision::Frontier,
            evmc_revision::EVMC_HOMESTEAD => Revision::Homestead,
            evmc_revision::EVMC_TANGERINE_WHISTLE => Revision::Tangerine,
            evmc_revision::EVMC_SPURIOUS_DRAGON => Revision::Spurious,
            evmc_revision::EVMC_BYZANTIUM => Revision::Byzantium,
            evmc_revision::EVMC_CONSTANTINOPLE => Revision::Constantinople,
            evmc_revision::EVMC_PETERSBURG => Revision::Petersburg,
            evmc_revision::EVMC_ISTANBUL => Revision::Istanbul,
            evmc_revision::EVMC_BERLIN => Revision::Berlin,
            evmc_revision::EVMC_LONDON => Revision::London,
            evmc_revision::EVMC_SHANGHAI => Revision::Shanghai,
        }
    }
}

impl From<Revision> for evmc_vm::Revision {
    fn from(rev: Revision) -> Self {
        match rev {
            Revision::Frontier => evmc_revision::EVMC_FRONTIER,
            Revision::Homestead => evmc_revision::EVMC_HOMESTEAD,
            Revision::Tangerine => evmc_revision::EVMC_TANGERINE_WHISTLE,
            Revision::Spurious => evmc_revision::EVMC_SPURIOUS_DRAGON,
            Revision::Byzantium => evmc_revision::EVMC_BYZANTIUM,
            Revision::Constantinople => evmc_revision::EVMC_CONSTANTINOPLE,
            Revision::Petersburg => evmc_revision::EVMC_PETERSBURG,
            Revision::Istanbul => evmc_revision::EVMC_ISTANBUL,
            Revision::Berlin => evmc_revision::EVMC_BERLIN,
            Revision::London => evmc_revision::EVMC_LONDON,
            Revision::Shanghai => evmc_revision::EVMC_SHANGHAI,
        }
    }
}

#[evmc_declare::evmc_declare_vm("evmodin", "evm", "0.1.0")]
pub struct EvmOdin;

impl EvmcVm for EvmOdin {
    fn init() -> Self {
        Self
    }

    fn execute<'a>(
        &self,
        revision: evmc_vm::Revision,
        code: &'a [u8],
        message: &'a ExecutionMessage,
        context: Option<&'a mut ExecutionContext<'a>>,
    ) -> ExecutionResult {
        let code = AnalyzedCode::analyze(code);

        let output = if let Some(context) = context {
            code.execute(
                context,
                NoopTracer,
                Message::from_evmc(&message),
                revision.into(),
            )
        } else {
            code.execute(
                &mut DummyHost,
                NoopTracer,
                Message::from_evmc(&message),
                revision.into(),
            )
        };

        ExecutionResult::new(
            output.status_code.clone().into(),
            output.gas_left,
            (!output.output_data.is_empty()).then(|| &*output.output_data),
        )
    }
}
