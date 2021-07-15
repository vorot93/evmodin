use super::*;
use crate::state::*;

pub(crate) fn load_push<const N: usize>(stack: &mut Stack, code: &[u8]) -> usize {
    stack.push(U256::from_big_endian(&code[..N]));
    N
}

pub(crate) fn dup<const N: usize>(stack: &mut Stack) {
    stack.push(*stack.get(N - 1));
}

pub(crate) fn swap<const N: usize>(stack: &mut Stack) {
    stack.swap_top(N);
}

pub(crate) fn pop(stack: &mut Stack) {
    stack.pop();
}
