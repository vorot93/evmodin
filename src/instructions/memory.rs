use crate::{common::*, state::*};
use ethereum_types::U256;
use sha3::{Digest, Keccak256};
use std::{cmp::min, num::NonZeroUsize};

pub(crate) const MAX_BUFFER_SIZE: u32 = u32::MAX;

/// The size of the EVM 256-bit word.
const WORD_SIZE: i64 = 32;

/// Returns number of words what would fit to provided number of bytes,
/// i.e. it rounds up the number bytes to number of words.
pub(crate) fn num_words(size_in_bytes: usize) -> i64 {
    ((size_in_bytes as i64) + (WORD_SIZE - 1)) / WORD_SIZE
}

pub(crate) fn mload(state: &mut ExecutionState) -> Result<(), StatusCode> {
    let index = state.stack.pop();

    let region = verify_memory_region_u64(state, index, NonZeroUsize::new(32).unwrap())
        .map(|region| region.unwrap())
        .map_err(|_| StatusCode::OutOfGas)?;

    let value =
        U256::from_big_endian(&state.memory[region.offset..region.offset + region.size.get()]);

    state.stack.push(value);

    Ok(())
}

pub(crate) fn mstore(state: &mut ExecutionState) -> Result<(), StatusCode> {
    let index = state.stack.pop();
    let value = state.stack.pop();

    let region = verify_memory_region_u64(state, index, NonZeroUsize::new(32).unwrap())
        .map(|region| region.unwrap())
        .map_err(|_| StatusCode::OutOfGas)?;

    let mut b = [0; 32];
    value.to_big_endian(&mut b);
    state.memory[region.offset..region.offset + 32].copy_from_slice(&b);

    Ok(())
}

pub(crate) fn mstore8(state: &mut ExecutionState) -> Result<(), StatusCode> {
    let index = state.stack.pop();
    let value = state.stack.pop();

    let region = verify_memory_region_u64(state, index, NonZeroUsize::new(1).unwrap())
        .map(|region| region.unwrap())
        .map_err(|_| StatusCode::OutOfGas)?;

    let value = (value.low_u32() & 0xff) as u8;

    state.memory[region.offset] = value;

    Ok(())
}

pub(crate) fn msize(state: &mut ExecutionState) {
    state.stack.push(state.memory.len().into());
}

pub(crate) fn verify_memory_region_u64(
    state: &mut ExecutionState,
    offset: U256,
    size: NonZeroUsize,
) -> Result<Option<MemoryRegion>, ()> {
    if offset > U256::from(MAX_BUFFER_SIZE) {
        return Err(());
    }

    let new_size = offset.as_usize() + size.get();
    let current_size = state.memory.len();
    if new_size > current_size {
        let new_words = num_words(new_size);
        let current_words = (current_size / 32) as i64;
        let new_cost = 3 * new_words + new_words * new_words / 512;
        let current_cost = 3 * current_words + current_words * current_words / 512;
        let cost = new_cost - current_cost;

        state.gas_left -= cost;

        if state.gas_left < 0 {
            return Err(());
        }

        state
            .memory
            .resize((new_words * WORD_SIZE) as usize, Default::default());
    }

    Ok(Some(MemoryRegion {
        offset: offset.as_usize(),
        size,
    }))
}

pub(crate) struct MemoryRegion {
    pub offset: usize,
    pub size: NonZeroUsize,
}

pub(crate) fn verify_memory_region(
    state: &mut ExecutionState,
    offset: U256,
    size: U256,
) -> Result<Option<MemoryRegion>, ()> {
    if size.is_zero() {
        return Ok(None);
    }

    if size > U256::from(MAX_BUFFER_SIZE) {
        return Err(());
    }

    verify_memory_region_u64(state, offset, NonZeroUsize::new(size.as_usize()).unwrap())
}

pub(crate) fn calldatacopy(state: &mut ExecutionState) -> Result<(), StatusCode> {
    let mem_index = state.stack.pop();
    let input_index = state.stack.pop();
    let size = state.stack.pop();

    let region = verify_memory_region(state, mem_index, size).map_err(|_| StatusCode::OutOfGas)?;

    if let Some(region) = &region {
        let copy_cost = num_words(region.size.get()) * 3;
        state.gas_left -= copy_cost;
        if state.gas_left < 0 {
            return Err(StatusCode::OutOfGas);
        }

        let input_len = state.message.input_data.len().into();

        let src = core::cmp::min(input_len, input_index).as_usize();
        let copy_size = core::cmp::min(size, input_len - src).as_usize();

        if copy_size > 0 {
            state.memory[region.offset..region.offset + copy_size]
                .copy_from_slice(&state.message.input_data[src..src + copy_size]);
        }

        if region.size.get() - copy_size > 0 {
            state.memory[region.offset + copy_size..region.offset + region.size.get()].fill(0);
        }
    }

    Ok(())
}

pub(crate) fn keccak256(state: &mut ExecutionState) -> Result<(), StatusCode> {
    let index = state.stack.pop();
    let size = state.stack.pop();

    let region = verify_memory_region(state, index, size).map_err(|_| StatusCode::OutOfGas)?;

    state.stack.push(U256::from_big_endian(&*Keccak256::digest(
        if let Some(region) = region {
            let w = num_words(region.size.get());
            let cost = w * 6;
            state.gas_left -= cost;
            if state.gas_left < 0 {
                return Err(StatusCode::OutOfGas);
            }

            &state.memory[region.offset..region.offset + region.size.get()]
        } else {
            &[]
        },
    )));

    Ok(())
}

pub(crate) fn codesize(stack: &mut Stack, code: &[u8]) {
    stack.push(code.len().into())
}

pub(crate) fn codecopy(state: &mut ExecutionState, code: &[u8]) -> Result<(), StatusCode> {
    // TODO: Similar to calldatacopy().

    let mem_index = state.stack.pop();
    let input_index = state.stack.pop();
    let size = state.stack.pop();

    let region = verify_memory_region(state, mem_index, size).map_err(|_| StatusCode::OutOfGas)?;

    if let Some(region) = region {
        let src = min(U256::from(code.len()), input_index).as_usize();
        let copy_size = min(region.size.get(), code.len() - src);

        let copy_cost = num_words(region.size.get()) * 3;
        state.gas_left -= copy_cost;
        if state.gas_left < 0 {
            return Err(StatusCode::OutOfGas);
        }

        // TODO: Add unit tests for each combination of conditions.
        if copy_size > 0 {
            state.memory[region.offset..region.offset + copy_size]
                .copy_from_slice(&code[src..src + copy_size]);
        }

        if region.size.get() - copy_size > 0 {
            state.memory[region.offset + copy_size..region.offset + region.size.get()].fill(0);
        }
    }

    Ok(())
}

#[doc(hidden)]
#[macro_export]
macro_rules! extcodecopy {
    ($co:expr, $state:expr) => {
        use crate::{
            common::*,
            continuation::{interrupt_data::*, resume_data::*},
            host::*,
            instructions::{memory::*, properties::*},
        };
        use core::cmp::min;

        let addr = u256_to_address($state.stack.pop());
        let mem_index = $state.stack.pop();
        let input_index = $state.stack.pop();
        let size = $state.stack.pop();

        let region =
            verify_memory_region($state, mem_index, size).map_err(|_| StatusCode::OutOfGas)?;

        if let Some(region) = &region {
            let copy_cost = num_words(region.size.get()) * 3;
            $state.gas_left -= copy_cost;
            if $state.gas_left < 0 {
                return Err(StatusCode::OutOfGas);
            }
        }

        if $state.evm_revision >= Revision::Berlin
            && ResumeDataVariant::into_access_account_status(
                $co.yield_(InterruptDataVariant::AccessAccount(AccessAccount {
                    address: addr,
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

        if let Some(region) = region {
            let src = min(U256::from(MAX_BUFFER_SIZE), input_index).as_usize();

            let r = &mut $state.memory[region.offset..region.offset + region.size.get()];
            let code = ResumeDataVariant::into_code(
                $co.yield_(InterruptDataVariant::CopyCode(CopyCode {
                    address: addr,
                    offset: src,
                    max_size: r.len(),
                }))
                .await,
            )
            .unwrap()
            .code;

            r[..code.len()].copy_from_slice(&code);
            if region.size.get() - code.len() > 0 {
                $state.memory[region.offset + code.len()..region.offset + region.size.get()]
                    .fill(0);
            }
        }
    };
}

pub(crate) fn returndatasize(state: &mut ExecutionState) {
    state.stack.push(state.return_data.len().into());
}

pub(crate) fn returndatacopy(state: &mut ExecutionState) -> Result<(), StatusCode> {
    let mem_index = state.stack.pop();
    let input_index = state.stack.pop();
    let size = state.stack.pop();

    let region = verify_memory_region(state, mem_index, size).map_err(|_| StatusCode::OutOfGas)?;

    if input_index > U256::from(state.return_data.len()) {
        return Err(StatusCode::InvalidMemoryAccess);
    }
    let src = input_index.as_usize();

    if src + region.as_ref().map(|r| r.size.get()).unwrap_or(0) > state.return_data.len() {
        return Err(StatusCode::InvalidMemoryAccess);
    }

    if let Some(region) = region {
        let copy_cost = num_words(region.size.get()) * 3;
        state.gas_left -= copy_cost;
        if state.gas_left < 0 {
            return Err(StatusCode::OutOfGas);
        }

        state.memory[region.offset..region.offset + region.size.get()]
            .copy_from_slice(&state.return_data[src..src + region.size.get()]);
    }

    Ok(())
}

#[doc(hidden)]
#[macro_export]
macro_rules! extcodehash {
    ($co:expr, $state:expr) => {
        use crate::{
            common::*,
            continuation::{interrupt_data::*, resume_data::*},
            host::*,
            instructions::properties::*,
        };

        let addr = u256_to_address($state.stack.pop());

        if $state.evm_revision >= Revision::Berlin
            && ResumeDataVariant::into_access_account_status(
                $co.yield_(InterruptDataVariant::AccessAccount(AccessAccount {
                    address: addr,
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

        $state.stack.push(U256::from_big_endian(
            ResumeDataVariant::into_code_hash(
                $co.yield_(InterruptDataVariant::GetCodeHash(GetCodeHash {
                    address: addr,
                }))
                .await,
            )
            .unwrap()
            .hash
            .as_bytes(),
        ));
    };
}
