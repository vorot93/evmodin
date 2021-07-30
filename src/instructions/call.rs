#[doc(hidden)]
#[macro_export]
macro_rules! do_call {
    ($co:expr, $state:expr, $kind:expr, $is_static:expr) => {{
        use std::cmp::min;
        use $crate::{
            common::u256_to_address,
            continuation::{interrupt_data::*, resume_data::*},
            host::AccessStatus,
            instructions::{memory::MemoryRegion, properties::*},
            CallKind, Message,
        };

        let gas = $state.stack.pop();
        let dst = u256_to_address($state.stack.pop());
        let value = if $is_static || matches!($kind, CallKind::DelegateCall) {
            U256::zero()
        } else {
            $state.stack.pop()
        };
        let has_value = !value.is_zero();
        let input_offset = $state.stack.pop();
        let input_size = $state.stack.pop();
        let output_offset = $state.stack.pop();
        let output_size = $state.stack.pop();

        $state.stack.push(U256::zero()); // Assume failure.

        if $state.evm_revision >= Revision::Berlin
            && ResumeDataVariant::into_access_account_status(
                $co.yield_(InterruptDataVariant::AccessAccount(AccessAccount {
                    address: dst,
                }))
                .await,
            )
            .unwrap()
            .status
                == AccessStatus::Cold
        {
            $state.gas_left -= i64::from(ADDITIONAL_COLD_ACCOUNT_ACCESS_COST);
            if $state.gas_left < 0 {
                return Err(StatusCode::OutOfGas);
            }
        }

        let input_region = memory::verify_memory_region($state, input_offset, input_size)
            .map_err(|_| StatusCode::OutOfGas)?;
        let output_region = memory::verify_memory_region($state, output_offset, output_size)
            .map_err(|_| StatusCode::OutOfGas)?;

        let mut msg = Message {
            kind: $kind,
            is_static: $is_static || $state.message.is_static,
            depth: $state.message.depth + 1,
            destination: dst,
            sender: if matches!($kind, CallKind::DelegateCall) {
                $state.message.sender
            } else {
                $state.message.destination
            },
            gas: i64::MAX,
            value: if matches!($kind, CallKind::DelegateCall) {
                $state.message.value
            } else {
                value
            },
            input_data: input_region
                .map(|MemoryRegion { offset, size }| {
                    $state.memory[offset..offset + size.get()].to_vec().into()
                })
                .unwrap_or_default(),
        };

        let mut cost = if has_value { 9000 } else { 0 };

        if matches!($kind, CallKind::Call) {
            if has_value && $state.message.is_static {
                return Err(StatusCode::StaticModeViolation);
            }

            if (has_value || $state.evm_revision < Revision::Spurious)
                && !ResumeDataVariant::into_account_exists_status(
                    $co.yield_(InterruptDataVariant::AccountExists(AccountExists {
                        address: dst,
                    }))
                    .await,
                )
                .unwrap()
                .exists
            {
                cost += 25000;
            }
        }
        $state.gas_left -= cost;
        if $state.gas_left < 0 {
            return Err(StatusCode::OutOfGas);
        }

        if gas < msg.gas.into() {
            msg.gas = gas.as_usize() as i64;
        }

        if $state.evm_revision >= Revision::Tangerine {
            // TODO: Always true for STATICCALL.
            msg.gas = min(msg.gas, $state.gas_left - $state.gas_left / 64);
        } else if msg.gas > $state.gas_left {
            return Err(StatusCode::OutOfGas);
        }

        if has_value {
            msg.gas += 2300; // Add stipend.
            $state.gas_left += 2300;
        }

        $state.return_data.clear();

        if $state.message.depth < 1024
            && !(has_value
                && ResumeDataVariant::into_balance(
                    $co.yield_(InterruptDataVariant::GetBalance(GetBalance {
                        address: $state.message.destination,
                    }))
                    .await,
                )
                .unwrap()
                .balance
                    < value)
        {
            let msg_gas = msg.gas;
            let result = ResumeDataVariant::into_call_output(
                $co.yield_(InterruptDataVariant::Call(Call { message: msg }))
                    .await,
            )
            .unwrap()
            .output;
            $state.return_data = result.output_data.clone();
            *$state.stack.get_mut(0) = if matches!(result.status_code, StatusCode::Success) {
                U256::one()
            } else {
                U256::zero()
            };

            if let Some(MemoryRegion { offset, size }) = output_region {
                let copy_size = min(size.get(), result.output_data.len());
                if copy_size > 0 {
                    $state.memory[offset..offset + copy_size]
                        .copy_from_slice(&result.output_data[..copy_size]);
                }
            }

            let gas_used = msg_gas - result.gas_left;
            $state.gas_left -= gas_used;
        }
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! do_create {
    ($co:expr, $state:expr, $create2:expr) => {{
        use ethereum_types::H256;
        use $crate::{
            common::*,
            continuation::{interrupt_data::*, resume_data::*},
            CallKind, Message,
        };

        if $state.message.is_static {
            return Err(StatusCode::StaticModeViolation);
        }

        let endowment = $state.stack.pop();
        let init_code_offset = $state.stack.pop();
        let init_code_size = $state.stack.pop();

        let region = memory::verify_memory_region($state, init_code_offset, init_code_size)
            .map_err(|_| StatusCode::OutOfGas)?;

        let call_kind = if $create2 {
            let salt = $state.stack.pop();

            if let Some(region) = &region {
                let salt_cost = memory::num_words(region.size.get()) * 6;
                $state.gas_left -= salt_cost;
                if $state.gas_left < 0 {
                    return Err(StatusCode::OutOfGas);
                }
            }

            CallKind::Create2 {
                salt: H256(salt.into()),
            }
        } else {
            CallKind::Create
        };

        $state.stack.push(U256::zero());
        $state.return_data.clear();

        if $state.message.depth < 1024
            && !(!endowment.is_zero()
                && ResumeDataVariant::into_balance(
                    $co.yield_(InterruptDataVariant::GetBalance(GetBalance {
                        address: $state.message.destination,
                    }))
                    .await,
                )
                .unwrap()
                .balance
                    < endowment)
        {
            let msg = Message {
                gas: if $state.evm_revision >= Revision::Tangerine {
                    $state.gas_left - $state.gas_left / 64
                } else {
                    $state.gas_left
                },

                is_static: false,
                destination: Address::zero(),

                kind: call_kind,
                input_data: if !init_code_size.is_zero() {
                    $state.memory[init_code_offset.as_usize()
                        ..init_code_offset.as_usize() + init_code_size.as_usize()]
                        .to_vec()
                        .into()
                } else {
                    Bytes::new()
                },
                sender: $state.message.destination,
                depth: $state.message.depth + 1,
                value: endowment,
            };
            let msg_gas = msg.gas;
            let result = ResumeDataVariant::into_call_output(
                $co.yield_(InterruptDataVariant::Call(Call { message: msg }))
                    .await,
            )
            .unwrap()
            .output;
            $state.gas_left -= msg_gas - result.gas_left;

            $state.return_data = result.output_data;
            if result.status_code == StatusCode::Success {
                *$state.stack.get_mut(0) =
                    address_to_u256(result.create_address.expect("expected create address"));
            }
        }
    }};
}
