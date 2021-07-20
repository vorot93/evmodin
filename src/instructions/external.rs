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
use ethereum_types::{Address, H256};
use genawaiter::{sync::*, *};
use std::{future::Future, pin::Pin};

pub(crate) fn address(state: &mut ExecutionState) {
    state.stack.push(address_to_u256(state.message.destination));
}

pub(crate) fn caller(state: &mut ExecutionState) {
    state.stack.push(address_to_u256(state.message.sender));
}

pub(crate) fn callvalue(state: &mut ExecutionState) {
    state.stack.push(state.message.value);
}

pub(crate) async fn balance<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> Result<(), StatusCode> {
    let address = u256_to_address(state.stack.pop());

    if state.evm_revision >= Revision::Berlin
        && host.access_account(address).await? == AccessStatus::Cold
    {
        state.gas_left -= i64::from(ADDITIONAL_COLD_ACCOUNT_ACCESS_COST);
        if state.gas_left < 0 {
            return Err(StatusCode::OutOfGas);
        }
    }

    state.stack.push(host.get_balance(address).await?);

    Ok(())
}

pub(crate) fn balance_gen(
    state: &mut ExecutionState,
) -> impl Coroutine<Yield = Interrupt, Resume = ResumeData, Return = Result<(), StatusCode>>
       + Send
       + Sync
       + Unpin
       + '_ {
    gen!({
        let address = u256_to_address(state.stack.pop());

        if state.evm_revision >= Revision::Berlin {
            let access_status = match yield_!(Interrupt::AccessAccount { address }) {
                ResumeData::AccessAccount { status } => status,
                _ => unreachable!(),
            };
            if access_status == AccessStatus::Cold {
                state.gas_left -= i64::from(ADDITIONAL_COLD_ACCOUNT_ACCESS_COST);
                if state.gas_left < 0 {
                    return Err(StatusCode::OutOfGas);
                }
            }
        }

        let balance = match yield_!(Interrupt::GetBalance { address }) {
            ResumeData::GetBalance { balance } => balance,
            _ => unreachable!(),
        };

        state.stack.push(balance);

        Ok(())
    })
}

pub(crate) async fn extcodesize<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> Result<(), StatusCode> {
    let address = u256_to_address(state.stack.pop());

    if state.evm_revision >= Revision::Berlin
        && host.access_account(address).await? == AccessStatus::Cold
    {
        state.gas_left -= i64::from(ADDITIONAL_COLD_ACCOUNT_ACCESS_COST);
        if state.gas_left < 0 {
            return Err(StatusCode::OutOfGas);
        }
    }

    state.stack.push(host.get_code_size(address).await?);

    Ok(())
}

pub(crate) async fn gasprice<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> anyhow::Result<()> {
    state.stack.push(host.get_tx_context().await?.tx_gas_price);
    Ok(())
}

pub(crate) async fn origin<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> anyhow::Result<()> {
    state
        .stack
        .push(address_to_u256(host.get_tx_context().await?.tx_origin));
    Ok(())
}

pub(crate) async fn coinbase<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> anyhow::Result<()> {
    state
        .stack
        .push(address_to_u256(host.get_tx_context().await?.block_coinbase));
    Ok(())
}

pub(crate) async fn number<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> anyhow::Result<()> {
    state
        .stack
        .push(host.get_tx_context().await?.block_number.into());
    Ok(())
}

pub(crate) async fn timestamp<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> anyhow::Result<()> {
    state
        .stack
        .push(host.get_tx_context().await?.block_timestamp.into());
    Ok(())
}

pub(crate) async fn gaslimit<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> anyhow::Result<()> {
    state
        .stack
        .push(host.get_tx_context().await?.block_gas_limit.into());
    Ok(())
}

pub(crate) async fn difficulty<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> anyhow::Result<()> {
    state
        .stack
        .push(host.get_tx_context().await?.block_difficulty);
    Ok(())
}

pub(crate) async fn chainid<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> anyhow::Result<()> {
    state.stack.push(host.get_tx_context().await?.chain_id);
    Ok(())
}

pub(crate) async fn basefee<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> anyhow::Result<()> {
    state
        .stack
        .push(host.get_tx_context().await?.block_base_fee);
    Ok(())
}

pub(crate) async fn selfbalance<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> anyhow::Result<()> {
    state
        .stack
        .push(host.get_balance(state.message.destination).await?);
    Ok(())
}

pub(crate) async fn blockhash<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> anyhow::Result<()> {
    let number = state.stack.pop();

    let upper_bound = host.get_tx_context().await?.block_number;
    let lower_bound = upper_bound.saturating_sub(256);

    let mut header = H256::zero();
    if number <= u64::MAX.into() {
        let n = number.as_u64();
        if (lower_bound..upper_bound).contains(&n) {
            header = host.get_block_hash(n).await?;
        }
    }

    state.stack.push(U256::from_big_endian(&header.0));

    Ok(())
}

pub(crate) async fn log<H: Host, const N: usize>(
    host: &mut H,
    state: &mut ExecutionState,
) -> Result<(), StatusCode> {
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

    let mut topics = [H256::zero(); N];
    for topic in &mut topics {
        *topic = H256(state.stack.pop().into());
    }

    let data = if let Some(region) = region {
        &state.memory[region.offset..region.offset + region.size.get()]
    } else {
        &[]
    };
    host.emit_log(state.message.destination, data, &topics)
        .await?;

    Ok(())
}

pub(crate) async fn sload<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> Result<(), StatusCode> {
    let key = H256(state.stack.pop().into());

    if state.evm_revision >= Revision::Berlin
        && host.access_storage(state.message.destination, key).await? == AccessStatus::Cold
    {
        // The warm storage access cost is already applied (from the cost table).
        // Here we need to apply additional cold storage access cost.
        const ADDITIONAL_COLD_SLOAD_COST: u16 = COLD_SLOAD_COST - WARM_STORAGE_READ_COST;
        state.gas_left -= i64::from(ADDITIONAL_COLD_SLOAD_COST);
        if state.gas_left < 0 {
            return Err(StatusCode::OutOfGas);
        }
    }

    state.stack.push(U256::from_big_endian(
        host.get_storage(state.message.destination, key)
            .await?
            .as_bytes(),
    ));

    Ok(())
}

pub(crate) async fn sstore<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> Result<(), StatusCode> {
    if state.message.is_static {
        return Err(StatusCode::StaticModeViolation);
    }

    if state.evm_revision >= Revision::Istanbul && state.gas_left <= 2300 {
        return Err(StatusCode::OutOfGas);
    }

    let key = H256(state.stack.pop().into());
    let value = H256(state.stack.pop().into());

    let mut cost = 0;
    if state.evm_revision >= Revision::Berlin
        && host.access_storage(state.message.destination, key).await? == AccessStatus::Cold
    {
        cost = COLD_SLOAD_COST;
    }

    let status = host
        .set_storage(state.message.destination, key, value)
        .await?;

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
}

pub(crate) async fn selfdestruct<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> Result<(), StatusCode> {
    if state.message.is_static {
        return Err(StatusCode::StaticModeViolation);
    }

    let beneficiary = u256_to_address(state.stack.pop());

    if state.evm_revision >= Revision::Berlin
        && host.access_account(beneficiary).await? == AccessStatus::Cold
    {
        state.gas_left -= i64::from(COLD_ACCOUNT_ACCESS_COST);
        if state.gas_left < 0 {
            return Err(StatusCode::OutOfGas);
        }
    }

    if state.evm_revision >= Revision::Tangerine
        && (state.evm_revision == Revision::Tangerine
            || !host.get_balance(state.message.destination).await?.is_zero())
    {
        // After TANGERINE_WHISTLE apply additional cost of
        // sending value to a non-existing account.
        if !host.account_exists(beneficiary).await? {
            state.gas_left -= 25000;
            if state.gas_left < 0 {
                return Err(StatusCode::OutOfGas);
            }
        }
    }

    host.selfdestruct(state.message.destination, beneficiary)
        .await?;
    Ok(())
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
