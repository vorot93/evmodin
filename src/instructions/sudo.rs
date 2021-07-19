use ethereum_types::H256;
use num_traits::Saturating;

use super::{
    properties::{COLD_ACCOUNT_ACCESS_COST, WARM_STORAGE_READ_COST},
    *,
};
use crate::{
    common::{address_to_u256, u256_to_address},
    host::*,
    instructions::properties::{ADDITIONAL_COLD_ACCOUNT_ACCESS_COST, COLD_SLOAD_COST},
    state::ExecutionState,
};

pub(crate) fn burn_gas(state: &mut ExecutionState) -> Result<(), StatusCode> {
    let gas = state.stack.pop();

    if gas > i64::MAX.into() {
        return Err(StatusCode::OutOfGas);
    }

    state.gas_left = state.gas_left.saturating_sub(gas.as_u64() as i64);

    if state.gas_left < 0 {
        return Err(StatusCode::OutOfGas);
    }

    Ok(())
}

pub(crate) async fn sub_balance<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> Result<(), StatusCode> {
    let address = u256_to_address(state.stack.pop());
    let amount = state.stack.pop();

    host.sub_balance(address, amount).await?;

    Ok(())
}

pub(crate) async fn add_balance<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> Result<(), StatusCode> {
    let address = u256_to_address(state.stack.pop());
    let balance = state.stack.pop();

    host.add_balance(address, balance).await?;

    Ok(())
}
