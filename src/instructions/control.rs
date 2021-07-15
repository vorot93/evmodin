use super::*;
use crate::{interpreter::JumpdestMap, StatusCode};

#[inline]
pub(crate) fn ret(state: &mut ExecutionState, status_code: StatusCode) -> InstructionResolution {
    let offset = *state.stack.get(0);
    let size = *state.stack.get(1);

    if let Some(region) = if let Ok(r) = super::memory::verify_memory_region(state, offset, size) {
        r
    } else {
        return InstructionResolution::Exit(StatusCode::OutOfGas);
    } {
        state.output_data = state.memory[region.offset..region.offset + region.size.get()]
            .to_vec()
            .into();
    }

    InstructionResolution::Exit(status_code)
}

pub(crate) fn op_jump(
    state: &mut ExecutionState,
    jumpdest_map: &JumpdestMap,
) -> InstructionResolution {
    let dst = state.stack.pop();
    if !jumpdest_map.contains(dst) {
        return InstructionResolution::Exit(StatusCode::BadJumpDestination);
    }

    InstructionResolution::Jump(dst.as_usize())
}

pub(crate) fn calldataload(state: &mut ExecutionState) {
    let index = state.stack.pop();

    let input_len = state.message.input_data.len();

    state.stack.push({
        if index > U256::from(input_len) {
            U256::zero()
        } else {
            let index_usize = index.as_usize();
            let end = core::cmp::min(index_usize + 32, input_len);

            let mut data = [0; 32];
            data[..end - index_usize].copy_from_slice(&state.message.input_data[index_usize..end]);

            data.into()
        }
    });
}

pub(crate) fn calldatasize(state: &mut ExecutionState) {
    state.stack.push(state.message.input_data.len().into());
}
