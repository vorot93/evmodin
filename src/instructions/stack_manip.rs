use crate::state::*;
use ethereum_types::U256;

pub(crate) fn push(stack: &mut Stack, code: &[u8], push_len: usize) {
    stack.push(U256::from_big_endian(&code[..push_len]));
}

pub(crate) fn dup(stack: &mut Stack, height: usize) {
    stack.push(*stack.get(height - 1));
}

pub(crate) fn swap(stack: &mut Stack, height: usize) {
    stack.swap_top(height);
}

pub(crate) fn pop(stack: &mut Stack) {
    stack.pop();
}
