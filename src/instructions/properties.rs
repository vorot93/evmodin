use once_cell::sync::Lazy;

use crate::{common::Revision, opcode::*};

pub(crate) const COLD_SLOAD_COST: u16 = 2100;
pub(crate) const COLD_ACCOUNT_ACCESS_COST: u16 = 2600;
pub(crate) const WARM_STORAGE_READ_COST: u16 = 100;

/// Additional cold account access cost.
///
/// The warm access cost is unconditionally applied for every account access instruction.
/// If the access turns out to be cold, this cost must be applied additionally.
pub(crate) const ADDITIONAL_COLD_ACCOUNT_ACCESS_COST: u16 =
    COLD_ACCOUNT_ACCESS_COST - WARM_STORAGE_READ_COST;

/// EVM instruction properties
#[derive(Clone, Copy, Debug)]
pub struct Properties {
    /// The instruction name
    pub name: &'static str,
    /// The number of stack items the instruction accesses during execution.
    pub stack_height_required: u8,
    /// The stack height change caused by the instruction execution. Can be negative.
    pub stack_height_change: i8,
}

impl Properties {
    fn new(name: &'static str, stack_height_required: u8, stack_height_change: i8) -> Self {
        Self {
            name,
            stack_height_required,
            stack_height_change,
        }
    }
}

pub static PROPERTIES: Lazy<[Option<Properties>; 256]> = Lazy::new(|| {
    let mut table = [None; 256];

    table[OpCode::STOP.to_usize()] = Some(Properties::new("STOP", 0, 0));
    table[OpCode::ADD.to_usize()] = Some(Properties::new("ADD", 2, -1));
    table[OpCode::MUL.to_usize()] = Some(Properties::new("MUL", 2, -1));
    table[OpCode::SUB.to_usize()] = Some(Properties::new("SUB", 2, -1));
    table[OpCode::DIV.to_usize()] = Some(Properties::new("DIV", 2, -1));
    table[OpCode::SDIV.to_usize()] = Some(Properties::new("SDIV", 2, -1));
    table[OpCode::MOD.to_usize()] = Some(Properties::new("MOD", 2, -1));
    table[OpCode::SMOD.to_usize()] = Some(Properties::new("SMOD", 2, -1));
    table[OpCode::ADDMOD.to_usize()] = Some(Properties::new("ADDMOD", 3, -2));
    table[OpCode::MULMOD.to_usize()] = Some(Properties::new("MULMOD", 3, -2));
    table[OpCode::EXP.to_usize()] = Some(Properties::new("EXP", 2, -1));
    table[OpCode::SIGNEXTEND.to_usize()] = Some(Properties::new("SIGNEXTEND", 2, -1));

    table[OpCode::LT.to_usize()] = Some(Properties::new("LT", 2, -1));
    table[OpCode::GT.to_usize()] = Some(Properties::new("GT", 2, -1));
    table[OpCode::SLT.to_usize()] = Some(Properties::new("SLT", 2, -1));
    table[OpCode::SGT.to_usize()] = Some(Properties::new("SGT", 2, -1));
    table[OpCode::EQ.to_usize()] = Some(Properties::new("EQ", 2, -1));
    table[OpCode::ISZERO.to_usize()] = Some(Properties::new("ISZERO", 1, 0));
    table[OpCode::AND.to_usize()] = Some(Properties::new("AND", 2, -1));
    table[OpCode::OR.to_usize()] = Some(Properties::new("OR", 2, -1));
    table[OpCode::XOR.to_usize()] = Some(Properties::new("XOR", 2, -1));
    table[OpCode::NOT.to_usize()] = Some(Properties::new("NOT", 1, 0));
    table[OpCode::BYTE.to_usize()] = Some(Properties::new("BYTE", 2, -1));
    table[OpCode::SHL.to_usize()] = Some(Properties::new("SHL", 2, -1));
    table[OpCode::SHR.to_usize()] = Some(Properties::new("SHR", 2, -1));
    table[OpCode::SAR.to_usize()] = Some(Properties::new("SAR", 2, -1));

    table[OpCode::KECCAK256.to_usize()] = Some(Properties::new("KECCAK256", 2, -1));

    table[OpCode::ADDRESS.to_usize()] = Some(Properties::new("ADDRESS", 0, 1));
    table[OpCode::BALANCE.to_usize()] = Some(Properties::new("BALANCE", 1, 0));
    table[OpCode::ORIGIN.to_usize()] = Some(Properties::new("ORIGIN", 0, 1));
    table[OpCode::CALLER.to_usize()] = Some(Properties::new("CALLER", 0, 1));
    table[OpCode::CALLVALUE.to_usize()] = Some(Properties::new("CALLVALUE", 0, 1));
    table[OpCode::CALLDATALOAD.to_usize()] = Some(Properties::new("CALLDATALOAD", 1, 0));
    table[OpCode::CALLDATASIZE.to_usize()] = Some(Properties::new("CALLDATASIZE", 0, 1));
    table[OpCode::CALLDATACOPY.to_usize()] = Some(Properties::new("CALLDATACOPY", 3, -3));
    table[OpCode::CODESIZE.to_usize()] = Some(Properties::new("CODESIZE", 0, 1));
    table[OpCode::CODECOPY.to_usize()] = Some(Properties::new("CODECOPY", 3, -3));
    table[OpCode::GASPRICE.to_usize()] = Some(Properties::new("GASPRICE", 0, 1));
    table[OpCode::EXTCODESIZE.to_usize()] = Some(Properties::new("EXTCODESIZE", 1, 0));
    table[OpCode::EXTCODECOPY.to_usize()] = Some(Properties::new("EXTCODECOPY", 4, -4));
    table[OpCode::RETURNDATASIZE.to_usize()] = Some(Properties::new("RETURNDATASIZE", 0, 1));
    table[OpCode::RETURNDATACOPY.to_usize()] = Some(Properties::new("RETURNDATACOPY", 3, -3));
    table[OpCode::EXTCODEHASH.to_usize()] = Some(Properties::new("EXTCODEHASH", 1, 0));

    table[OpCode::BLOCKHASH.to_usize()] = Some(Properties::new("BLOCKHASH", 1, 0));
    table[OpCode::COINBASE.to_usize()] = Some(Properties::new("COINBASE", 0, 1));
    table[OpCode::TIMESTAMP.to_usize()] = Some(Properties::new("TIMESTAMP", 0, 1));
    table[OpCode::NUMBER.to_usize()] = Some(Properties::new("NUMBER", 0, 1));
    table[OpCode::DIFFICULTY.to_usize()] = Some(Properties::new("DIFFICULTY", 0, 1));
    table[OpCode::GASLIMIT.to_usize()] = Some(Properties::new("GASLIMIT", 0, 1));
    table[OpCode::CHAINID.to_usize()] = Some(Properties::new("CHAINID", 0, 1));
    table[OpCode::SELFBALANCE.to_usize()] = Some(Properties::new("SELFBALANCE", 0, 1));
    table[OpCode::BASEFEE.to_usize()] = Some(Properties::new("BASEFEE", 0, 1));

    table[OpCode::POP.to_usize()] = Some(Properties::new("POP", 1, -1));
    table[OpCode::MLOAD.to_usize()] = Some(Properties::new("MLOAD", 1, 0));
    table[OpCode::MSTORE.to_usize()] = Some(Properties::new("MSTORE", 2, -2));
    table[OpCode::MSTORE8.to_usize()] = Some(Properties::new("MSTORE8", 2, -2));
    table[OpCode::SLOAD.to_usize()] = Some(Properties::new("SLOAD", 1, 0));
    table[OpCode::SSTORE.to_usize()] = Some(Properties::new("SSTORE", 2, -2));
    table[OpCode::JUMP.to_usize()] = Some(Properties::new("JUMP", 1, -1));
    table[OpCode::JUMPI.to_usize()] = Some(Properties::new("JUMPI", 2, -2));
    table[OpCode::PC.to_usize()] = Some(Properties::new("PC", 0, 1));
    table[OpCode::MSIZE.to_usize()] = Some(Properties::new("MSIZE", 0, 1));
    table[OpCode::GAS.to_usize()] = Some(Properties::new("GAS", 0, 1));
    table[OpCode::JUMPDEST.to_usize()] = Some(Properties::new("JUMPDEST", 0, 0));

    table[OpCode::PUSH1.to_usize()] = Some(Properties::new("PUSH1", 0, 1));
    table[OpCode::PUSH2.to_usize()] = Some(Properties::new("PUSH2", 0, 1));
    table[OpCode::PUSH3.to_usize()] = Some(Properties::new("PUSH3", 0, 1));
    table[OpCode::PUSH4.to_usize()] = Some(Properties::new("PUSH4", 0, 1));
    table[OpCode::PUSH5.to_usize()] = Some(Properties::new("PUSH5", 0, 1));
    table[OpCode::PUSH6.to_usize()] = Some(Properties::new("PUSH6", 0, 1));
    table[OpCode::PUSH7.to_usize()] = Some(Properties::new("PUSH7", 0, 1));
    table[OpCode::PUSH8.to_usize()] = Some(Properties::new("PUSH8", 0, 1));
    table[OpCode::PUSH9.to_usize()] = Some(Properties::new("PUSH9", 0, 1));
    table[OpCode::PUSH10.to_usize()] = Some(Properties::new("PUSH10", 0, 1));
    table[OpCode::PUSH11.to_usize()] = Some(Properties::new("PUSH11", 0, 1));
    table[OpCode::PUSH12.to_usize()] = Some(Properties::new("PUSH12", 0, 1));
    table[OpCode::PUSH13.to_usize()] = Some(Properties::new("PUSH13", 0, 1));
    table[OpCode::PUSH14.to_usize()] = Some(Properties::new("PUSH14", 0, 1));
    table[OpCode::PUSH15.to_usize()] = Some(Properties::new("PUSH15", 0, 1));
    table[OpCode::PUSH16.to_usize()] = Some(Properties::new("PUSH16", 0, 1));
    table[OpCode::PUSH17.to_usize()] = Some(Properties::new("PUSH17", 0, 1));
    table[OpCode::PUSH18.to_usize()] = Some(Properties::new("PUSH18", 0, 1));
    table[OpCode::PUSH19.to_usize()] = Some(Properties::new("PUSH19", 0, 1));
    table[OpCode::PUSH20.to_usize()] = Some(Properties::new("PUSH20", 0, 1));
    table[OpCode::PUSH21.to_usize()] = Some(Properties::new("PUSH21", 0, 1));
    table[OpCode::PUSH22.to_usize()] = Some(Properties::new("PUSH22", 0, 1));
    table[OpCode::PUSH23.to_usize()] = Some(Properties::new("PUSH23", 0, 1));
    table[OpCode::PUSH24.to_usize()] = Some(Properties::new("PUSH24", 0, 1));
    table[OpCode::PUSH25.to_usize()] = Some(Properties::new("PUSH25", 0, 1));
    table[OpCode::PUSH26.to_usize()] = Some(Properties::new("PUSH26", 0, 1));
    table[OpCode::PUSH27.to_usize()] = Some(Properties::new("PUSH27", 0, 1));
    table[OpCode::PUSH28.to_usize()] = Some(Properties::new("PUSH28", 0, 1));
    table[OpCode::PUSH29.to_usize()] = Some(Properties::new("PUSH29", 0, 1));
    table[OpCode::PUSH30.to_usize()] = Some(Properties::new("PUSH30", 0, 1));
    table[OpCode::PUSH31.to_usize()] = Some(Properties::new("PUSH31", 0, 1));
    table[OpCode::PUSH32.to_usize()] = Some(Properties::new("PUSH32", 0, 1));

    table[OpCode::DUP1.to_usize()] = Some(Properties::new("DUP1", 1, 1));
    table[OpCode::DUP2.to_usize()] = Some(Properties::new("DUP2", 2, 1));
    table[OpCode::DUP3.to_usize()] = Some(Properties::new("DUP3", 3, 1));
    table[OpCode::DUP4.to_usize()] = Some(Properties::new("DUP4", 4, 1));
    table[OpCode::DUP5.to_usize()] = Some(Properties::new("DUP5", 5, 1));
    table[OpCode::DUP6.to_usize()] = Some(Properties::new("DUP6", 6, 1));
    table[OpCode::DUP7.to_usize()] = Some(Properties::new("DUP7", 7, 1));
    table[OpCode::DUP8.to_usize()] = Some(Properties::new("DUP8", 8, 1));
    table[OpCode::DUP9.to_usize()] = Some(Properties::new("DUP9", 9, 1));
    table[OpCode::DUP10.to_usize()] = Some(Properties::new("DUP10", 10, 1));
    table[OpCode::DUP11.to_usize()] = Some(Properties::new("DUP11", 11, 1));
    table[OpCode::DUP12.to_usize()] = Some(Properties::new("DUP12", 12, 1));
    table[OpCode::DUP13.to_usize()] = Some(Properties::new("DUP13", 13, 1));
    table[OpCode::DUP14.to_usize()] = Some(Properties::new("DUP14", 14, 1));
    table[OpCode::DUP15.to_usize()] = Some(Properties::new("DUP15", 15, 1));
    table[OpCode::DUP16.to_usize()] = Some(Properties::new("DUP16", 16, 1));

    table[OpCode::SWAP1.to_usize()] = Some(Properties::new("SWAP1", 2, 0));
    table[OpCode::SWAP2.to_usize()] = Some(Properties::new("SWAP2", 3, 0));
    table[OpCode::SWAP3.to_usize()] = Some(Properties::new("SWAP3", 4, 0));
    table[OpCode::SWAP4.to_usize()] = Some(Properties::new("SWAP4", 5, 0));
    table[OpCode::SWAP5.to_usize()] = Some(Properties::new("SWAP5", 6, 0));
    table[OpCode::SWAP6.to_usize()] = Some(Properties::new("SWAP6", 7, 0));
    table[OpCode::SWAP7.to_usize()] = Some(Properties::new("SWAP7", 8, 0));
    table[OpCode::SWAP8.to_usize()] = Some(Properties::new("SWAP8", 9, 0));
    table[OpCode::SWAP9.to_usize()] = Some(Properties::new("SWAP9", 10, 0));
    table[OpCode::SWAP10.to_usize()] = Some(Properties::new("SWAP10", 11, 0));
    table[OpCode::SWAP11.to_usize()] = Some(Properties::new("SWAP11", 12, 0));
    table[OpCode::SWAP12.to_usize()] = Some(Properties::new("SWAP12", 13, 0));
    table[OpCode::SWAP13.to_usize()] = Some(Properties::new("SWAP13", 14, 0));
    table[OpCode::SWAP14.to_usize()] = Some(Properties::new("SWAP14", 15, 0));
    table[OpCode::SWAP15.to_usize()] = Some(Properties::new("SWAP15", 16, 0));
    table[OpCode::SWAP16.to_usize()] = Some(Properties::new("SWAP16", 17, 0));

    table[OpCode::LOG0.to_usize()] = Some(Properties::new("LOG0", 2, -2));
    table[OpCode::LOG1.to_usize()] = Some(Properties::new("LOG1", 3, -3));
    table[OpCode::LOG2.to_usize()] = Some(Properties::new("LOG2", 4, -4));
    table[OpCode::LOG3.to_usize()] = Some(Properties::new("LOG3", 5, -5));
    table[OpCode::LOG4.to_usize()] = Some(Properties::new("LOG4", 6, -6));

    table[OpCode::CREATE.to_usize()] = Some(Properties::new("CREATE", 3, -2));
    table[OpCode::CALL.to_usize()] = Some(Properties::new("CALL", 7, -6));
    table[OpCode::CALLCODE.to_usize()] = Some(Properties::new("CALLCODE", 7, -6));
    table[OpCode::RETURN.to_usize()] = Some(Properties::new("RETURN", 2, -2));
    table[OpCode::DELEGATECALL.to_usize()] = Some(Properties::new("DELEGATECALL", 6, -5));
    table[OpCode::CREATE2.to_usize()] = Some(Properties::new("CREATE2", 4, -3));
    table[OpCode::STATICCALL.to_usize()] = Some(Properties::new("STATICCALL", 6, -5));
    table[OpCode::REVERT.to_usize()] = Some(Properties::new("REVERT", 2, -2));
    table[OpCode::INVALID.to_usize()] = Some(Properties::new("INVALID", 0, 0));
    table[OpCode::SELFDESTRUCT.to_usize()] = Some(Properties::new("SELFDESTRUCT", 1, -1));

    table
});

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

static HOMESTEAD_GAS_COSTS: Lazy<[Option<u16>; 256]> = Lazy::new(|| {
    let mut table = *FRONTIER_GAS_COSTS;
    table[OpCode::DELEGATECALL.to_usize()] = Some(40);
    table
});

static TANGERINE_GAS_COSTS: Lazy<[Option<u16>; 256]> = Lazy::new(|| {
    let mut table = *HOMESTEAD_GAS_COSTS;
    table[OpCode::BALANCE.to_usize()] = Some(400);
    table[OpCode::EXTCODESIZE.to_usize()] = Some(700);
    table[OpCode::EXTCODECOPY.to_usize()] = Some(700);
    table[OpCode::SLOAD.to_usize()] = Some(200);
    table[OpCode::CALL.to_usize()] = Some(700);
    table[OpCode::CALLCODE.to_usize()] = Some(700);
    table[OpCode::DELEGATECALL.to_usize()] = Some(700);
    table[OpCode::SELFDESTRUCT.to_usize()] = Some(5000);
    table
});

static SPURIOUS_GAS_COSTS: Lazy<[Option<u16>; 256]> = Lazy::new(|| *TANGERINE_GAS_COSTS);

static BYZANTIUM_GAS_COSTS: Lazy<[Option<u16>; 256]> = Lazy::new(|| {
    let mut table = *SPURIOUS_GAS_COSTS;
    table[OpCode::RETURNDATASIZE.to_usize()] = Some(2);
    table[OpCode::RETURNDATACOPY.to_usize()] = Some(3);
    table[OpCode::STATICCALL.to_usize()] = Some(700);
    table[OpCode::REVERT.to_usize()] = Some(0);
    table
});

static CONSTANTINOPLE_GAS_COSTS: Lazy<[Option<u16>; 256]> = Lazy::new(|| {
    let mut table = *BYZANTIUM_GAS_COSTS;
    table[OpCode::SHL.to_usize()] = Some(3);
    table[OpCode::SHR.to_usize()] = Some(3);
    table[OpCode::SAR.to_usize()] = Some(3);
    table[OpCode::EXTCODEHASH.to_usize()] = Some(400);
    table[OpCode::CREATE2.to_usize()] = Some(32000);
    table
});

static PETERSBURG_GAS_COSTS: Lazy<[Option<u16>; 256]> = Lazy::new(|| *CONSTANTINOPLE_GAS_COSTS);

static ISTANBUL_GAS_COSTS: Lazy<[Option<u16>; 256]> = Lazy::new(|| {
    let mut table = *PETERSBURG_GAS_COSTS;
    table[OpCode::BALANCE.to_usize()] = Some(700);
    table[OpCode::CHAINID.to_usize()] = Some(2);
    table[OpCode::EXTCODEHASH.to_usize()] = Some(700);
    table[OpCode::SELFBALANCE.to_usize()] = Some(5);
    table[OpCode::SLOAD.to_usize()] = Some(800);
    table
});

static BERLIN_GAS_COSTS: Lazy<[Option<u16>; 256]> = Lazy::new(|| {
    let mut table = *ISTANBUL_GAS_COSTS;
    table[OpCode::EXTCODESIZE.to_usize()] = Some(WARM_STORAGE_READ_COST);
    table[OpCode::EXTCODECOPY.to_usize()] = Some(WARM_STORAGE_READ_COST);
    table[OpCode::EXTCODEHASH.to_usize()] = Some(WARM_STORAGE_READ_COST);
    table[OpCode::BALANCE.to_usize()] = Some(WARM_STORAGE_READ_COST);
    table[OpCode::CALL.to_usize()] = Some(WARM_STORAGE_READ_COST);
    table[OpCode::CALLCODE.to_usize()] = Some(WARM_STORAGE_READ_COST);
    table[OpCode::DELEGATECALL.to_usize()] = Some(WARM_STORAGE_READ_COST);
    table[OpCode::STATICCALL.to_usize()] = Some(WARM_STORAGE_READ_COST);
    table[OpCode::SLOAD.to_usize()] = Some(WARM_STORAGE_READ_COST);
    table
});

static LONDON_GAS_COSTS: Lazy<[Option<u16>; 256]> = Lazy::new(|| {
    let mut table = *BERLIN_GAS_COSTS;
    table[OpCode::BASEFEE.to_usize()] = Some(2);
    table
});

pub fn gas_costs(revision: Revision) -> &'static [Option<u16>; 256] {
    match revision {
        Revision::Frontier => &FRONTIER_GAS_COSTS,
        Revision::Homestead => &HOMESTEAD_GAS_COSTS,
        Revision::Tangerine => &TANGERINE_GAS_COSTS,
        Revision::Spurious => &SPURIOUS_GAS_COSTS,
        Revision::Byzantium => &BYZANTIUM_GAS_COSTS,
        Revision::Constantinople => &CONSTANTINOPLE_GAS_COSTS,
        Revision::Petersburg => &PETERSBURG_GAS_COSTS,
        Revision::Istanbul => &ISTANBUL_GAS_COSTS,
        Revision::Berlin => &BERLIN_GAS_COSTS,
        Revision::London => &LONDON_GAS_COSTS,
        Revision::Shanghai => &LONDON_GAS_COSTS,
    }
}
