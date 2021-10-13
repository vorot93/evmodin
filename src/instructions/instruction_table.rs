use crate::{instructions::properties, Config, OpCode, Revision};
use once_cell::sync::Lazy;

#[allow(clippy::needless_range_loop)]
static FRONTIER_GAS_COSTS: Lazy<[Option<u16>; 256]> = Lazy::new(|| {
    let mut table = [None; 256];

    table[OpCode::STOP.to_usize()] = Some(0);
    table[OpCode::ADD.to_usize()] = Some(3);
    table[OpCode::MUL.to_usize()] = Some(5);
    table[OpCode::SUB.to_usize()] = Some(3);
    table[OpCode::DIV.to_usize()] = Some(5);
    table[OpCode::SDIV.to_usize()] = Some(5);
    table[OpCode::MOD.to_usize()] = Some(5);
    table[OpCode::SMOD.to_usize()] = Some(5);
    table[OpCode::ADDMOD.to_usize()] = Some(8);
    table[OpCode::MULMOD.to_usize()] = Some(8);
    table[OpCode::EXP.to_usize()] = Some(10);
    table[OpCode::SIGNEXTEND.to_usize()] = Some(5);
    table[OpCode::LT.to_usize()] = Some(3);
    table[OpCode::GT.to_usize()] = Some(3);
    table[OpCode::SLT.to_usize()] = Some(3);
    table[OpCode::SGT.to_usize()] = Some(3);
    table[OpCode::EQ.to_usize()] = Some(3);
    table[OpCode::ISZERO.to_usize()] = Some(3);
    table[OpCode::AND.to_usize()] = Some(3);
    table[OpCode::OR.to_usize()] = Some(3);
    table[OpCode::XOR.to_usize()] = Some(3);
    table[OpCode::NOT.to_usize()] = Some(3);
    table[OpCode::BYTE.to_usize()] = Some(3);
    table[OpCode::KECCAK256.to_usize()] = Some(30);
    table[OpCode::ADDRESS.to_usize()] = Some(2);
    table[OpCode::BALANCE.to_usize()] = Some(20);
    table[OpCode::ORIGIN.to_usize()] = Some(2);
    table[OpCode::CALLER.to_usize()] = Some(2);
    table[OpCode::CALLVALUE.to_usize()] = Some(2);
    table[OpCode::CALLDATALOAD.to_usize()] = Some(3);
    table[OpCode::CALLDATASIZE.to_usize()] = Some(2);
    table[OpCode::CALLDATACOPY.to_usize()] = Some(3);
    table[OpCode::CODESIZE.to_usize()] = Some(2);
    table[OpCode::CODECOPY.to_usize()] = Some(3);
    table[OpCode::GASPRICE.to_usize()] = Some(2);
    table[OpCode::EXTCODESIZE.to_usize()] = Some(20);
    table[OpCode::EXTCODECOPY.to_usize()] = Some(20);
    table[OpCode::BLOCKHASH.to_usize()] = Some(20);
    table[OpCode::COINBASE.to_usize()] = Some(2);
    table[OpCode::TIMESTAMP.to_usize()] = Some(2);
    table[OpCode::NUMBER.to_usize()] = Some(2);
    table[OpCode::DIFFICULTY.to_usize()] = Some(2);
    table[OpCode::GASLIMIT.to_usize()] = Some(2);
    table[OpCode::POP.to_usize()] = Some(2);
    table[OpCode::MLOAD.to_usize()] = Some(3);
    table[OpCode::MSTORE.to_usize()] = Some(3);
    table[OpCode::MSTORE8.to_usize()] = Some(3);
    table[OpCode::SLOAD.to_usize()] = Some(50);
    table[OpCode::SSTORE.to_usize()] = Some(0);
    table[OpCode::JUMP.to_usize()] = Some(8);
    table[OpCode::JUMPI.to_usize()] = Some(10);
    table[OpCode::PC.to_usize()] = Some(2);
    table[OpCode::MSIZE.to_usize()] = Some(2);

    table[OpCode::GAS.to_usize()] = Some(2);
    table[OpCode::JUMPDEST.to_usize()] = Some(1);

    for op in OpCode::PUSH1.to_usize()..=OpCode::PUSH32.to_usize() {
        table[op] = Some(3);
    }

    for op in OpCode::DUP1.to_usize()..=OpCode::DUP16.to_usize() {
        table[op] = Some(3);
    }

    for op in OpCode::SWAP1.to_usize()..=OpCode::SWAP16.to_usize() {
        table[op] = Some(3);
    }

    for (i, op) in (OpCode::LOG0.to_usize()..=OpCode::LOG4.to_usize())
        .into_iter()
        .enumerate()
    {
        table[op] = Some((1 + i as u16) * 375);
    }

    table[OpCode::CREATE.to_usize()] = Some(32000);
    table[OpCode::CALL.to_usize()] = Some(40);
    table[OpCode::CALLCODE.to_usize()] = Some(40);
    table[OpCode::RETURN.to_usize()] = Some(0);
    table[OpCode::INVALID.to_usize()] = Some(0);
    table[OpCode::SELFDESTRUCT.to_usize()] = Some(0);

    table
});

#[derive(Clone, Copy, Debug)]
pub struct InstructionTableEntry {
    /// None = disabled
    pub gas_cost: Option<u16>,
    pub stack_height_required: u8,
    pub can_overflow_stack: bool,
}

pub type InstructionTable = [Option<InstructionTableEntry>; 256];

pub static BASE_INSTRUCTION_TABLE: Lazy<InstructionTable> = Lazy::new(|| {
    let mut table = [None; 256];

    for (opcode, &property) in properties::PROPERTIES.iter().enumerate() {
        if let Some(property) = property {
            let stack_height_required = property.stack_height_required;

            table[opcode] = Some(InstructionTableEntry {
                gas_cost: FRONTIER_GAS_COSTS[opcode],
                stack_height_required,
                can_overflow_stack: property.stack_height_change > 0,
            });
        }
    }
    table
});

pub fn build_instruction_table(config: &Config) -> InstructionTable {
    let mut table = *BASE_INSTRUCTION_TABLE;

    table[OpCode::EXTCODESIZE] = Some(config.gas_ext_code_size);
    table[OpCode::EXTCODECOPY] = Some(config.gas_ext_code_copy);
    table[OpCode::EXTCODEHASH] = Some(config.gas_ext_code_hash);
    table[OpCode::BALANCE] = Some(config.gas_balance);
    table[OpCode::SLOAD] = Some(config.gas_sload);
    table[OpCode::SELFDESTRUCT] = Some(config.gas_suicide);
    /// Gas paid for CALL opcode.
    pub gas_call: u64,
    /// Gas paid for EXP opcode for every byte.
    pub gas_expbyte: u64,
    /// Gas paid for a contract creation transaction.
    pub gas_transaction_create: u64,
    /// Gas paid for a message call transaction.
    pub gas_transaction_call: u64,
    /// Gas paid for zero data in a transaction.
    pub gas_transaction_zero_data: u64,
    /// Gas paid for non-zero data in a transaction.
    pub gas_transaction_non_zero_data: u64,
    /// Gas create divisor.
    pub gas_create_divisor: Option<u64>,
    /// EIP-1283.
    pub sstore_gas_metering: bool,
    /// EIP-1706.
    pub sstore_revert_under_stipend: bool,
    /// Whether to throw out of gas error when
    /// CALL/CALLCODE/DELEGATECALL requires more than maximum amount
    /// of gas.
    pub err_on_call_with_more_gas: bool,
    /// Whether create transactions and create opcode increases nonce by one.
    pub create_increase_nonce: bool,
    /// Stack limit.
    pub stack_limit: usize,
    /// Memory limit.
    pub memory_limit: usize,
    /// Call limit.
    pub call_stack_limit: usize,
    /// Create contract limit.
    pub create_contract_limit: Option<usize>,
    /// Call stipend.
    pub call_stipend: u64,
    /// Has delegate call.
    pub has_delegate_call: bool,
    /// Has create2.
    pub has_create2: bool,
    /// Has revert.
    pub has_revert: bool,
    /// Has return data.
    pub has_return_data: bool,
    /// Has static call.
    pub has_static_call: bool,
    /// Has bitwise shifting.
    pub has_bitwise_shifting: bool,
    /// Has chain ID.
    pub has_chain_id: bool,
    /// Has self balance.
    pub has_self_balance: bool,
    /// Has ext code hash.
    pub has_ext_code_hash: bool,
    /// Access list support.
    pub has_access_list: bool,
    /// Base fee support.
    pub has_base_fee: bool,

    table
}
