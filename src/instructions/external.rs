use super::{
    properties::{COLD_ACCOUNT_ACCESS_COST, WARM_STORAGE_READ_COST},
    *,
};
use crate::{
    common::{address_to_u256, u256_to_address},
    continuation::*,
    host::*,
    instructions::properties::{ADDITIONAL_COLD_ACCOUNT_ACCESS_COST, COLD_SLOAD_COST},
    state::ExecutionState,
};
use arrayvec::ArrayVec;
use ethereum_types::H256;
use genawaiter::{sync::*, *};

pub(crate) fn address(state: &mut ExecutionState) {
    state.stack.push(address_to_u256(state.message.destination));
}

pub(crate) fn caller(state: &mut ExecutionState) {
    state.stack.push(address_to_u256(state.message.sender));
}

pub(crate) fn callvalue(state: &mut ExecutionState) {
    state.stack.push(state.message.value);
}

pub(crate) fn balance(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = Result<(), StatusCode>>
       + Send
       + Sync
       + Unpin
       + '_ {
    gen!({
        let address = u256_to_address(state.stack.pop());

        if state.evm_revision >= Revision::Berlin {
            let access_status =
                ResumeData::into_access_account(yield_!(Interrupt::AccessAccount { address }))
                    .unwrap();
            if access_status == AccessStatus::Cold {
                state.gas_left -= i64::from(ADDITIONAL_COLD_ACCOUNT_ACCESS_COST);
                if state.gas_left < 0 {
                    return Err(StatusCode::OutOfGas);
                }
            }
        }

        let balance = ResumeData::into_balance(yield_!(Interrupt::GetBalance { address })).unwrap();

        state.stack.push(balance);

        Ok(())
    })
}

pub(crate) fn extcodesize(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = Result<(), StatusCode>>
       + Send
       + Sync
       + Unpin
       + '_ {
    gen!({
        let address = u256_to_address(state.stack.pop());

        if state.evm_revision >= Revision::Berlin {
            let access_account =
                ResumeData::into_access_account(yield_!(Interrupt::AccessAccount { address }))
                    .unwrap();
            if access_account == AccessStatus::Cold {
                state.gas_left -= i64::from(ADDITIONAL_COLD_ACCOUNT_ACCESS_COST);
                if state.gas_left < 0 {
                    return Err(StatusCode::OutOfGas);
                }
            }
        }

        let code_size =
            ResumeData::into_code_size(yield_!(Interrupt::GetCodeSize { address })).unwrap();
        state.stack.push(code_size);

        Ok(())
    })
}

pub(crate) fn gasprice(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = ()> + Send + Sync + Unpin + '_
{
    gen!({
        let tx_context = ResumeData::into_tx_context(yield_!(Interrupt::GetTxContext)).unwrap();

        state.stack.push(tx_context.tx_gas_price);
    })
}

pub(crate) fn origin(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = ()> + Send + Sync + Unpin + '_
{
    gen!({
        let tx_context = ResumeData::into_tx_context(yield_!(Interrupt::GetTxContext)).unwrap();

        state.stack.push(address_to_u256(tx_context.tx_origin));
    })
}

pub(crate) fn coinbase(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = ()> + Send + Sync + Unpin + '_
{
    gen!({
        let tx_context = ResumeData::into_tx_context(yield_!(Interrupt::GetTxContext)).unwrap();

        state.stack.push(address_to_u256(tx_context.block_coinbase));
    })
}

pub(crate) fn number(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = ()> + Send + Sync + Unpin + '_
{
    gen!({
        let tx_context = ResumeData::into_tx_context(yield_!(Interrupt::GetTxContext)).unwrap();

        state.stack.push(tx_context.block_number.into());
    })
}

pub(crate) fn timestamp(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = ()> + Send + Sync + Unpin + '_
{
    gen!({
        let tx_context = ResumeData::into_tx_context(yield_!(Interrupt::GetTxContext)).unwrap();

        state.stack.push(tx_context.block_timestamp.into());
    })
}

pub(crate) fn gaslimit(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = ()> + Send + Sync + Unpin + '_
{
    gen!({
        let tx_context = ResumeData::into_tx_context(yield_!(Interrupt::GetTxContext)).unwrap();

        state.stack.push(tx_context.block_gas_limit.into());
    })
}

pub(crate) fn difficulty(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = ()> + Send + Sync + Unpin + '_
{
    gen!({
        let tx_context = ResumeData::into_tx_context(yield_!(Interrupt::GetTxContext)).unwrap();

        state.stack.push(tx_context.block_difficulty);
    })
}

pub(crate) fn chainid(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = ()> + Send + Sync + Unpin + '_
{
    gen!({
        let tx_context = ResumeData::into_tx_context(yield_!(Interrupt::GetTxContext)).unwrap();

        state.stack.push(tx_context.chain_id);
    })
}

pub(crate) fn basefee(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = ()> + Send + Sync + Unpin + '_
{
    gen!({
        let tx_context = ResumeData::into_tx_context(yield_!(Interrupt::GetTxContext)).unwrap();

        state.stack.push(tx_context.block_base_fee);
    })
}

pub(crate) fn selfbalance(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = ()> + Send + Sync + Unpin + '_
{
    gen!({
        let balance = ResumeData::into_balance(yield_!(Interrupt::GetBalance {
            address: state.message.destination
        }))
        .unwrap();

        state.stack.push(balance);
    })
}

pub(crate) fn blockhash(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = ()> + Send + Sync + Unpin + '_
{
    gen!({
        let number = state.stack.pop();

        let upper_bound = ResumeData::into_tx_context(yield_!(Interrupt::GetTxContext))
            .unwrap()
            .block_number;
        let lower_bound = upper_bound.saturating_sub(256);

        let mut header = H256::zero();
        if number <= u64::MAX.into() {
            let n = number.as_u64();
            if (lower_bound..upper_bound).contains(&n) {
                header = ResumeData::into_block_hash(yield_!(Interrupt::GetBlockHash {
                    block_number: n
                }))
                .unwrap();
            }
        }

        state.stack.push(U256::from_big_endian(&header.0));
    })
}

pub(crate) fn log(
    state: &mut ExecutionState,
    num_topics: usize,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = Result<(), StatusCode>>
       + Send
       + Sync
       + Unpin
       + '_ {
    gen!({
        if state.message.is_static {
            return Err(StatusCode::StaticModeViolation);
        }

        let offset = state.stack.pop();
        let size = state.stack.pop();

        let region = if let Ok(r) = memory::verify_memory_region(state, offset, size) {
            r
        } else {
            return Err(StatusCode::OutOfGas);
        };

        if let Some(region) = &region {
            let cost = region.size.get() as i64 * 8;
            state.gas_left -= cost;
            if cost < 0 {
                return Err(StatusCode::OutOfGas);
            }
        }

        let mut topics = ArrayVec::new();
        for _ in 0..num_topics {
            topics.push(H256(state.stack.pop().into()));
        }

        let data = if let Some(region) = region {
            &state.memory[region.offset..region.offset + region.size.get()]
        } else {
            &[]
        };
        let r = yield_!(Interrupt::EmitLog {
            address: state.message.destination,
            data: data.to_vec().into(),
            topics,
        });

        assert!(matches!(r, ResumeData::Empty));

        Ok(())
    })
}

pub(crate) fn sload(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = Result<(), StatusCode>>
       + Send
       + Sync
       + Unpin
       + '_ {
    gen!({
        let key = H256(state.stack.pop().into());

        if state.evm_revision >= Revision::Berlin {
            let access_status =
                ResumeData::into_access_storage(yield_!(Interrupt::AccessStorage {
                    address: state.message.destination,
                    key,
                }))
                .unwrap();
            if access_status == AccessStatus::Cold {
                // The warm storage access cost is already applied (from the cost table).
                // Here we need to apply additional cold storage access cost.
                const ADDITIONAL_COLD_SLOAD_COST: u16 = COLD_SLOAD_COST - WARM_STORAGE_READ_COST;
                state.gas_left -= i64::from(ADDITIONAL_COLD_SLOAD_COST);
                if state.gas_left < 0 {
                    return Err(StatusCode::OutOfGas);
                }
            }
        }

        let storage = ResumeData::into_storage_value(yield_!(Interrupt::GetStorage {
            address: state.message.destination,
            key,
        }))
        .unwrap();

        state.stack.push(U256::from_big_endian(storage.as_bytes()));

        Ok(())
    })
}

pub(crate) fn sstore(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = Result<(), StatusCode>>
       + Send
       + Sync
       + Unpin
       + '_ {
    gen!({
        if state.message.is_static {
            return Err(StatusCode::StaticModeViolation);
        }

        if state.evm_revision >= Revision::Istanbul && state.gas_left <= 2300 {
            return Err(StatusCode::OutOfGas);
        }

        let key = H256(state.stack.pop().into());
        let value = H256(state.stack.pop().into());

        let mut cost = 0;
        if state.evm_revision >= Revision::Berlin {
            let access_status =
                ResumeData::into_access_storage(yield_!(Interrupt::AccessStorage {
                    address: state.message.destination,
                    key,
                }))
                .unwrap();

            if access_status == AccessStatus::Cold {
                cost = COLD_SLOAD_COST;
            }
        }

        let status = ResumeData::into_storage_status(yield_!(Interrupt::SetStorage {
            address: state.message.destination,
            key,
            value,
        }))
        .unwrap();

        cost = match status {
            StorageStatus::Unchanged | StorageStatus::ModifiedAgain => {
                if state.evm_revision >= Revision::Berlin {
                    cost + WARM_STORAGE_READ_COST
                } else if state.evm_revision == Revision::Istanbul {
                    800
                } else if state.evm_revision == Revision::Constantinople {
                    200
                } else {
                    5000
                }
            }
            StorageStatus::Modified | StorageStatus::Deleted => {
                if state.evm_revision >= Revision::Berlin {
                    cost + 5000 - COLD_SLOAD_COST
                } else {
                    5000
                }
            }
            StorageStatus::Added => cost + 20000,
        };
        state.gas_left -= i64::from(cost);
        if state.gas_left < 0 {
            return Err(StatusCode::OutOfGas);
        }
        Ok(())
    })
}

pub(crate) fn selfdestruct(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = Result<(), StatusCode>>
       + Send
       + Sync
       + Unpin
       + '_ {
    Gen::new(|mut co| async move {
        if state.message.is_static {
            return Err(StatusCode::StaticModeViolation);
        }

        let beneficiary = u256_to_address(state.stack.pop());

        if state.evm_revision >= Revision::Berlin {
            let access_status = ResumeData::into_access_account(
                co.yield_(Interrupt::AccessAccount {
                    address: beneficiary,
                })
                .await,
            )
            .unwrap();
            if access_status == AccessStatus::Cold {
                state.gas_left -= i64::from(COLD_ACCOUNT_ACCESS_COST);
                if state.gas_left < 0 {
                    return Err(StatusCode::OutOfGas);
                }
            }
        }

        if state.evm_revision >= Revision::Tangerine
            && (state.evm_revision == Revision::Tangerine
                || !{
                    ResumeData::into_balance(
                        co.yield_(Interrupt::GetBalance {
                            address: state.message.destination,
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
                co.yield_(Interrupt::AccountExists {
                    address: beneficiary,
                })
                .await,
            )
            .unwrap()
            {
                state.gas_left -= 25000;
                if state.gas_left < 0 {
                    return Err(StatusCode::OutOfGas);
                }
            }
        }

        assert!(matches!(
            co.yield_(Interrupt::Selfdestruct {
                address: state.message.destination,
                beneficiary,
            })
            .await,
            ResumeData::Empty
        ));

        Ok(())
    })
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
