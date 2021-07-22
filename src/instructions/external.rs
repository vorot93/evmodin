use crate::{common::address_to_u256, host::*, state::ExecutionState};
use ethereum_types::U256;

pub(crate) fn address(state: &mut ExecutionState) {
    state.stack.push(address_to_u256(state.message.destination));
}

pub(crate) fn caller(state: &mut ExecutionState) {
    state.stack.push(address_to_u256(state.message.sender));
}

pub(crate) fn callvalue(state: &mut ExecutionState) {
    state.stack.push(state.message.value);
}

#[macro_export]
macro_rules! balance {
    ($co:expr, $state:expr) => {
        use crate::{common::*, continuation::*, host::*, instructions::properties::*};

        let address = u256_to_address($state.stack.pop());

        if $state.evm_revision >= Revision::Berlin {
            let access_status = ResumeData::into_access_account(
                $co.yield_(Interrupt::AccessAccount { address }).await,
            )
            .unwrap();
            if access_status == AccessStatus::Cold {
                $state.gas_left -= i64::from(ADDITIONAL_COLD_ACCOUNT_ACCESS_COST);
                if $state.gas_left < 0 {
                    return Err(StatusCode::OutOfGas);
                }
            }
        }

        let balance =
            ResumeData::into_balance($co.yield_(Interrupt::GetBalance { address }).await).unwrap();

        $state.stack.push(balance);
    };
}

#[macro_export]
macro_rules! extcodesize {
    ($co:expr, $state:expr) => {
        use crate::{common::*, continuation::*, host::*, instructions::properties::*};

        let address = u256_to_address($state.stack.pop());

        if $state.evm_revision >= Revision::Berlin {
            let access_account = ResumeData::into_access_account(
                $co.yield_(Interrupt::AccessAccount { address }).await,
            )
            .unwrap();
            if access_account == AccessStatus::Cold {
                $state.gas_left -= i64::from(ADDITIONAL_COLD_ACCOUNT_ACCESS_COST);
                if $state.gas_left < 0 {
                    return Err(StatusCode::OutOfGas);
                }
            }
        }

        let code_size =
            ResumeData::into_code_size($co.yield_(Interrupt::GetCodeSize { address }).await)
                .unwrap();
        $state.stack.push(code_size);
    };
}

#[macro_export]
macro_rules! push_txcontext {
    ($co:expr, $state:expr, $accessor:expr) => {
        let tx_context =
            ResumeData::into_tx_context($co.yield_(Interrupt::GetTxContext).await).unwrap();

        $state.stack.push($accessor(tx_context));
    };
}

pub(crate) fn origin_accessor(tx_context: TxContext) -> U256 {
    address_to_u256(tx_context.tx_origin)
}

pub(crate) fn coinbase_accessor(tx_context: TxContext) -> U256 {
    address_to_u256(tx_context.block_coinbase)
}

pub(crate) fn gasprice_accessor(tx_context: TxContext) -> U256 {
    tx_context.tx_gas_price
}

pub(crate) fn timestamp_accessor(tx_context: TxContext) -> U256 {
    tx_context.block_timestamp.into()
}

pub(crate) fn number_accessor(tx_context: TxContext) -> U256 {
    tx_context.block_number.into()
}

pub(crate) fn gaslimit_accessor(tx_context: TxContext) -> U256 {
    tx_context.block_gas_limit.into()
}

pub(crate) fn difficulty_accessor(tx_context: TxContext) -> U256 {
    tx_context.block_difficulty
}

pub(crate) fn chainid_accessor(tx_context: TxContext) -> U256 {
    tx_context.chain_id
}

pub(crate) fn basefee_accessor(tx_context: TxContext) -> U256 {
    tx_context.block_base_fee
}

#[macro_export]
macro_rules! selfbalance {
    ($co:expr, $state:expr) => {{
        let balance = ResumeData::into_balance(
            $co.yield_(Interrupt::GetBalance {
                address: $state.message.destination,
            })
            .await,
        )
        .unwrap();

        $state.stack.push(balance);
    }};
}

#[macro_export]
macro_rules! blockhash {
    ($co:expr, $state:expr) => {
        use ethereum_types::H256;

        let number = $state.stack.pop();

        let upper_bound = ResumeData::into_tx_context($co.yield_(Interrupt::GetTxContext).await)
            .unwrap()
            .block_number;
        let lower_bound = upper_bound.saturating_sub(256);

        let mut header = H256::zero();
        if number <= u64::MAX.into() {
            let n = number.as_u64();
            if (lower_bound..upper_bound).contains(&n) {
                header = ResumeData::into_block_hash(
                    $co.yield_(Interrupt::GetBlockHash { block_number: n })
                        .await,
                )
                .unwrap();
            }
        }

        $state.stack.push(U256::from_big_endian(&header.0));
    };
}

#[macro_export]
macro_rules! do_log {
    ($co:expr, $state:expr, $num_topics:expr) => {{
        use arrayvec::ArrayVec;
        use ethereum_types::H256;

        if $state.message.is_static {
            return Err(StatusCode::StaticModeViolation);
        }

        let offset = $state.stack.pop();
        let size = $state.stack.pop();

        let region =
            memory::verify_memory_region($state, offset, size).map_err(|_| StatusCode::OutOfGas)?;

        if let Some(region) = &region {
            let cost = region.size.get() as i64 * 8;
            $state.gas_left -= cost;
            if cost < 0 {
                return Err(StatusCode::OutOfGas);
            }
        }

        let mut topics = ArrayVec::new();
        for _ in 0..$num_topics {
            topics.push(H256($state.stack.pop().into()));
        }

        let data = if let Some(region) = region {
            &$state.memory[region.offset..region.offset + region.size.get()]
        } else {
            &[]
        };
        let r = $co
            .yield_(Interrupt::EmitLog {
                address: $state.message.destination,
                data: data.to_vec().into(),
                topics,
            })
            .await;

        assert!(matches!(r, ResumeData::Empty));
    }};
}

#[macro_export]
macro_rules! sload {
    ($co:expr, $state:expr) => {{
        use ethereum_types::H256;
        use $crate::{
            continuation::*,
            host::*,
            instructions::properties::{COLD_SLOAD_COST, WARM_STORAGE_READ_COST},
        };

        let key = H256($state.stack.pop().into());

        if $state.evm_revision >= Revision::Berlin {
            let access_status = ResumeData::into_access_storage(
                $co.yield_(Interrupt::AccessStorage {
                    address: $state.message.destination,
                    key,
                })
                .await,
            )
            .unwrap();
            if access_status == AccessStatus::Cold {
                // The warm storage access cost is already applied (from the cost table).
                // Here we need to apply additional cold storage access cost.
                const ADDITIONAL_COLD_SLOAD_COST: u16 = COLD_SLOAD_COST - WARM_STORAGE_READ_COST;
                $state.gas_left -= i64::from(ADDITIONAL_COLD_SLOAD_COST);
                if $state.gas_left < 0 {
                    return Err(StatusCode::OutOfGas);
                }
            }
        }

        let storage = ResumeData::into_storage_value(
            $co.yield_(Interrupt::GetStorage {
                address: $state.message.destination,
                key,
            })
            .await,
        )
        .unwrap();

        $state.stack.push(U256::from_big_endian(storage.as_bytes()));
    }};
}

#[macro_export]
macro_rules! sstore {
    ($co:expr, $state:expr) => {{
        use ethereum_types::H256;
        use $crate::{
            continuation::*,
            host::*,
            instructions::properties::{COLD_SLOAD_COST, WARM_STORAGE_READ_COST},
        };

        if $state.message.is_static {
            return Err(StatusCode::StaticModeViolation);
        }

        if $state.evm_revision >= Revision::Istanbul && $state.gas_left <= 2300 {
            return Err(StatusCode::OutOfGas);
        }

        let key = H256($state.stack.pop().into());
        let value = H256($state.stack.pop().into());

        let mut cost = 0;
        if $state.evm_revision >= Revision::Berlin {
            let access_status = ResumeData::into_access_storage(
                $co.yield_(Interrupt::AccessStorage {
                    address: $state.message.destination,
                    key,
                })
                .await,
            )
            .unwrap();

            if access_status == AccessStatus::Cold {
                cost = COLD_SLOAD_COST;
            }
        }

        let status = ResumeData::into_storage_status(
            $co.yield_(Interrupt::SetStorage {
                address: $state.message.destination,
                key,
                value,
            })
            .await,
        )
        .unwrap();

        cost = match status {
            StorageStatus::Unchanged | StorageStatus::ModifiedAgain => {
                if $state.evm_revision >= Revision::Berlin {
                    cost + WARM_STORAGE_READ_COST
                } else if $state.evm_revision == Revision::Istanbul {
                    800
                } else if $state.evm_revision == Revision::Constantinople {
                    200
                } else {
                    5000
                }
            }
            StorageStatus::Modified | StorageStatus::Deleted => {
                if $state.evm_revision >= Revision::Berlin {
                    cost + 5000 - COLD_SLOAD_COST
                } else {
                    5000
                }
            }
            StorageStatus::Added => cost + 20000,
        };
        $state.gas_left -= i64::from(cost);
        if $state.gas_left < 0 {
            return Err(StatusCode::OutOfGas);
        }
    }};
}

#[macro_export]
macro_rules! selfdestruct {
    ($co:expr, $state:expr) => {{
        use crate::{common::*, continuation::*, host::*, instructions::properties::*};

        if $state.message.is_static {
            return Err(StatusCode::StaticModeViolation);
        }

        let beneficiary = u256_to_address($state.stack.pop());

        if $state.evm_revision >= Revision::Berlin {
            let access_status = ResumeData::into_access_account(
                $co.yield_(Interrupt::AccessAccount {
                    address: beneficiary,
                })
                .await,
            )
            .unwrap();
            if access_status == AccessStatus::Cold {
                $state.gas_left -= i64::from(COLD_ACCOUNT_ACCESS_COST);
                if $state.gas_left < 0 {
                    return Err(StatusCode::OutOfGas);
                }
            }
        }

        if $state.evm_revision >= Revision::Tangerine
            && ($state.evm_revision == Revision::Tangerine
                || !{
                    ResumeData::into_balance(
                        $co.yield_(Interrupt::GetBalance {
                            address: $state.message.destination,
                        })
                        .await,
                    )
                    .unwrap()
                    .is_zero()
                })
        {
            // After TANGERINE_WHISTLE apply additional cost of
            // sending value to a non-existing account.
            if !ResumeData::into_account_exists(
                $co.yield_(Interrupt::AccountExists {
                    address: beneficiary,
                })
                .await,
            )
            .unwrap()
            {
                $state.gas_left -= 25000;
                if $state.gas_left < 0 {
                    return Err(StatusCode::OutOfGas);
                }
            }
        }

        assert!(matches!(
            $co.yield_(Interrupt::Selfdestruct {
                address: $state.message.destination,
                beneficiary,
            })
            .await,
            ResumeData::Empty
        ));
    }};
}

#[cfg(test)]
mod tests {
    use crate::common::u256_to_address;
    use ethereum_types::Address;
    use hex_literal::hex;

    #[test]
    fn u256_to_address_conversion() {
        assert_eq!(
            u256_to_address(0x42.into()),
            Address::from(hex!("0000000000000000000000000000000000000042"))
        );
    }
}
