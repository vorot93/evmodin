use crate::{instructions::properties::WARM_STORAGE_READ_COST, Revision};

/// Runtime configuration.
#[derive(Clone, Debug)]
pub struct Config {
    /// Gas paid for extcode.
    pub gas_ext_code: u64,
    /// Gas paid for extcodecopy
    pub gas_ext_code_copy: u64,
    /// Gas paid for extcodehash.
    pub gas_ext_code_hash: u64,
    /// Gas paid for sstore set.
    pub gas_sstore_set: u64,
    /// Gas paid for sstore reset.
    pub gas_sstore_reset: u64,
    /// Gas paid for sstore refund.
    pub refund_sstore_clears: i64,
    /// Gas paid for BALANCE opcode.
    pub gas_balance: u64,
    /// Gas paid for SLOAD opcode.
    pub gas_sload: u64,
    /// Gas paid for SUICIDE opcode.
    pub gas_suicide: u64,
    /// Gas paid for SUICIDE opcode when it hits a new account.
    pub gas_suicide_new_account: u64,
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
}

impl Config {
    /// Frontier hard fork configuration.
    pub const fn frontier() -> Self {
        Self {
            gas_ext_code: 20,
            gas_ext_code_copy: 20,
            gas_ext_code_hash: 20,
            gas_balance: 20,
            gas_sload: 50,
            gas_sstore_set: 20000,
            gas_sstore_reset: 5000,
            refund_sstore_clears: 15000,
            gas_suicide: 0,
            gas_suicide_new_account: 0,
            gas_call: 40,
            gas_expbyte: 10,
            gas_transaction_create: 21000,
            gas_transaction_call: 21000,
            gas_transaction_zero_data: 4,
            gas_transaction_non_zero_data: 68,
            gas_create_divisor: None,
            sstore_gas_metering: false,
            sstore_revert_under_stipend: false,
            err_on_call_with_more_gas: true,
            create_increase_nonce: false,
            stack_limit: 1024,
            memory_limit: usize::MAX,
            call_stack_limit: 1024,
            create_contract_limit: None,
            call_stipend: 2300,
            has_delegate_call: false,
            has_create2: false,
            has_revert: false,
            has_return_data: false,
            has_static_call: false,
            has_bitwise_shifting: false,
            has_chain_id: false,
            has_self_balance: false,
            has_ext_code_hash: false,
            has_access_list: false,
            has_base_fee: false,
        }
    }

    /// Homestead hard fork configuration
    pub const fn homestead() -> Self {
        Self {
            gas_transaction_create: 53000,
            has_delegate_call: true,
            ..Self::frontier()
        }
    }

    /// Tangerine hard fork configuration.
    pub const fn tangerine() -> Self {
        Self {
            gas_ext_code: 700,
            gas_ext_code_copy: 700,
            gas_balance: 400,
            gas_sload: 200,
            gas_call: 700,
            gas_suicide: 5000,
            gas_suicide_new_account: 25000,
            gas_create_divisor: Some(64),
            ..Self::homestead()
        }
    }

    /// Spurious hard fork configuration.
    pub const fn spurious() -> Self {
        Self {
            create_contract_limit: Some(0x6000),
            ..Self::tangerine()
        }
    }

    pub const fn byzantium() -> Self {
        Self {
            has_revert: true,
            has_return_data: true,
            has_static_call: true,
            ..Self::spurious()
        }
    }

    pub const fn constantinople() -> Self {
        Self {
            has_bitwise_shifting: true,
            has_create2: true,
            has_ext_code_hash: true,
            sstore_gas_metering: true,
            ..Self::byzantium()
        }
    }

    pub const fn petersburg() -> Self {
        Self {
            sstore_gas_metering: false,
            ..Self::constantinople()
        }
    }

    pub const fn istanbul() -> Self {
        Self {
            sstore_gas_metering: true,
            sstore_revert_under_stipend: true,
            has_chain_id: true,
            gas_sload: 800,
            gas_balance: 700,
            gas_ext_code_hash: 700,
            has_self_balance: true,
            ..Self::petersburg()
        }
    }

    pub const fn berlin() -> Self {
        Self {
            gas_ext_code: WARM_STORAGE_READ_COST as u64,
            gas_ext_code_copy: WARM_STORAGE_READ_COST as u64,
            gas_ext_code_hash: WARM_STORAGE_READ_COST as u64,
            gas_balance: WARM_STORAGE_READ_COST as u64,
            gas_call: WARM_STORAGE_READ_COST as u64,
            gas_sload: WARM_STORAGE_READ_COST as u64,
            has_access_list: true,
            ..Self::istanbul()
        }
    }

    pub const fn london() -> Self {
        Self {
            has_base_fee: true,
            ..Self::berlin()
        }
    }

    pub const fn shanghai() -> Self {
        Self { ..Self::london() }
    }
}

impl From<Revision> for Config {
    fn from(rev: Revision) -> Self {
        match rev {
            Revision::Frontier => Self::frontier(),
            Revision::Homestead => Self::homestead(),
            Revision::Tangerine => Self::tangerine(),
            Revision::Spurious => Self::spurious(),
            Revision::Byzantium => Self::byzantium(),
            Revision::Constantinople => Self::constantinople(),
            Revision::Petersburg => Self::petersburg(),
            Revision::Istanbul => Self::istanbul(),
            Revision::Berlin => Self::berlin(),
            Revision::London => Self::london(),
            Revision::Shanghai => Self::shanghai(),
        }
    }
}
