use super::{properties::*, *};
use crate::{common::u256_to_address, host::AccessStatus, state::Stack, Host};
use sha3::{Digest, Keccak256};
use std::{cmp::min, num::NonZeroUsize};

const MAX_BUFFER_SIZE: u32 = u32::MAX;

/// The size of the EVM 256-bit word.
const WORD_SIZE: i64 = 32;

/// Returns number of words what would fit to provided number of bytes,
/// i.e. it rounds up the number bytes to number of words.
#[inline]
pub(crate) fn num_words(size_in_bytes: usize) -> i64 {
    ((size_in_bytes as i64) + (WORD_SIZE - 1)) / WORD_SIZE
}

#[inline]
pub(crate) fn mload(state: &mut ExecutionState) -> StatusCode {
    let index = state.stack.pop();

    let region = if let Ok(r) =
        verify_memory_region_u64(state, index, NonZeroUsize::new(32).unwrap())
            .map(|region| region.unwrap())
    {
        r
    } else {
        return StatusCode::OutOfGas;
    };

    let value =
        U256::from_big_endian(&state.memory[region.offset..region.offset + region.size.get()]);

    state.stack.push(value);

    StatusCode::Success
}

#[inline]
pub(crate) fn mstore(state: &mut ExecutionState) -> StatusCode {
    let index = state.stack.pop();
    let value = state.stack.pop();

    let region = if let Ok(r) =
        verify_memory_region_u64(state, index, NonZeroUsize::new(32).unwrap())
            .map(|region| region.unwrap())
    {
        r
    } else {
        return StatusCode::OutOfGas;
    };

    let mut b = [0; 32];
    value.to_big_endian(&mut b);
    state.memory[region.offset..region.offset + 32].copy_from_slice(&b);
    StatusCode::Success
}

#[inline]
pub(crate) fn mstore8(state: &mut ExecutionState) -> StatusCode {
    let index = state.stack.pop();
    let value = state.stack.pop();

    let region = if let Ok(r) =
        verify_memory_region_u64(state, index, NonZeroUsize::new(1).unwrap())
            .map(|region| region.unwrap())
    {
        r
    } else {
        return StatusCode::OutOfGas;
    };

    let value = (value.low_u32() & 0xff) as u8;

    state.memory[region.offset] = value;
    StatusCode::Success
}

#[inline]
pub(crate) fn msize(state: &mut ExecutionState) {
    state.stack.push(state.memory.len().into());
}

#[inline]

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

#[inline]
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

#[inline]
pub(crate) fn calldatacopy(state: &mut ExecutionState) -> InstructionResolution {
    let mem_index = state.stack.pop();
    let input_index = state.stack.pop();
    let size = state.stack.pop();

    let region = if let Ok(region) = verify_memory_region(state, mem_index, size) {
        region
    } else {
        return InstructionResolution::Exit(StatusCode::OutOfGas);
    };

    if let Some(region) = &region {
        let copy_cost = num_words(region.size.get()) * 3;
        state.gas_left -= copy_cost;
        if state.gas_left < 0 {
            return InstructionResolution::Exit(StatusCode::OutOfGas);
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

    InstructionResolution::Continue
}

#[inline]
pub(crate) fn keccak256(state: &mut ExecutionState) -> InstructionResolution {
    let index = state.stack.pop();
    let size = state.stack.pop();

    let region = if let Ok(region) = verify_memory_region(state, index, size) {
        region
    } else {
        return InstructionResolution::Exit(StatusCode::OutOfGas);
    };

    state.stack.push(U256::from_big_endian(&*Keccak256::digest(
        if let Some(region) = region {
            let w = num_words(region.size.get());
            let cost = w * 6;
            state.gas_left -= cost;
            if state.gas_left < 0 {
                return InstructionResolution::Exit(StatusCode::OutOfGas);
            }

            &state.memory[region.offset..region.offset + region.size.get()]
        } else {
            &[]
        },
    )));

    InstructionResolution::Continue
}

#[inline]
pub(crate) fn codesize(stack: &mut Stack, code: &[u8]) {
    stack.push(code.len().into())
}

#[inline]
pub(crate) fn codecopy(state: &mut ExecutionState, code: &[u8]) -> InstructionResolution {
    // TODO: Similar to calldatacopy().

    let mem_index = state.stack.pop();
    let input_index = state.stack.pop();
    let size = state.stack.pop();

    let region = if let Ok(r) = verify_memory_region(state, mem_index, size) {
        r
    } else {
        return InstructionResolution::Exit(StatusCode::OutOfGas);
    };

    if let Some(region) = region {
        let src = min(U256::from(code.len()), input_index).as_usize();
        let copy_size = min(region.size.get(), code.len() - src);

        let copy_cost = num_words(region.size.get()) * 3;
        state.gas_left -= copy_cost;
        if state.gas_left < 0 {
            return InstructionResolution::Exit(StatusCode::OutOfGas);
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

    InstructionResolution::Continue
}

#[inline]
pub(crate) async fn extcodecopy<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> anyhow::Result<InstructionResolution> {
    let addr = u256_to_address(state.stack.pop());
    let mem_index = state.stack.pop();
    let input_index = state.stack.pop();
    let size = state.stack.pop();

    let region = if let Ok(r) = verify_memory_region(state, mem_index, size) {
        r
    } else {
        return Ok(InstructionResolution::Exit(StatusCode::OutOfGas));
    };

    if let Some(region) = &region {
        let copy_cost = num_words(region.size.get()) * 3;
        state.gas_left -= copy_cost;
        if state.gas_left < 0 {
            return Ok(InstructionResolution::Exit(StatusCode::OutOfGas));
        }
    }

    if state.evm_revision >= Revision::Berlin
        && host.access_account(addr).await? == AccessStatus::Cold
    {
        state.gas_left -= i64::from(ADDITIONAL_COLD_ACCOUNT_ACCESS_COST);
        if state.gas_left < 0 {
            return Ok(InstructionResolution::Exit(StatusCode::OutOfGas));
        }
    }

    if let Some(region) = region {
        let src = min(U256::from(MAX_BUFFER_SIZE), input_index).as_usize();

        let num_bytes_copied = host
            .copy_code(
                addr,
                src,
                &mut state.memory[region.offset..region.offset + region.size.get()],
            )
            .await?;
        if region.size.get() - num_bytes_copied > 0 {
            state.memory[region.offset + num_bytes_copied..region.offset + region.size.get()]
                .fill(0);
        }
    }
    Ok(InstructionResolution::Continue)
}

#[inline]
pub(crate) fn returndatasize(state: &mut ExecutionState) {
    state.stack.push(state.return_data.len().into());
}

#[inline]
pub(crate) fn returndatacopy(state: &mut ExecutionState) -> InstructionResolution {
    let mem_index = state.stack.pop();
    let input_index = state.stack.pop();
    let size = state.stack.pop();

    let region = if let Ok(r) = verify_memory_region(state, mem_index, size) {
        r
    } else {
        return InstructionResolution::Exit(StatusCode::OutOfGas);
    };

    if input_index > U256::from(state.return_data.len()) {
        return InstructionResolution::Exit(StatusCode::InvalidMemoryAccess);
    }
    let src = input_index.as_usize();

    if src + region.as_ref().map(|r| r.size.get()).unwrap_or(0) > state.return_data.len() {
        return InstructionResolution::Exit(StatusCode::InvalidMemoryAccess);
    }

    if let Some(region) = region {
        let copy_cost = num_words(region.size.get()) * 3;
        state.gas_left -= copy_cost;
        if state.gas_left < 0 {
            return InstructionResolution::Exit(StatusCode::OutOfGas);
        }

        state.memory[region.offset..region.offset + region.size.get()]
            .copy_from_slice(&state.return_data[src..src + region.size.get()]);
    }

    InstructionResolution::Continue
}

#[inline]
pub(crate) async fn extcodehash<H: Host>(
    host: &mut H,
    state: &mut ExecutionState,
) -> anyhow::Result<InstructionResolution> {
    let addr = u256_to_address(state.stack.pop());

    if state.evm_revision >= Revision::Berlin
        && host.access_account(addr).await? == AccessStatus::Cold
    {
        state.gas_left -= i64::from(ADDITIONAL_COLD_ACCOUNT_ACCESS_COST);
        if state.gas_left < 0 {
            return Ok(InstructionResolution::Exit(StatusCode::OutOfGas));
        }
    }

    state
        .stack
        .push(U256::from_big_endian(&host.get_code_hash(addr).await?.0));
    Ok(InstructionResolution::Continue)
}
