use crate::state::*;
use ethereum_types::U256;
use i256::I256;

#[inline]
pub(crate) fn lt(stack: &mut Stack) {
    let a = stack.pop();
    let b = stack.pop();

    stack.push(if a.lt(&b) { U256::one() } else { U256::zero() })
}

#[inline]
pub(crate) fn gt(stack: &mut Stack) {
    let a = stack.pop();
    let b = stack.pop();

    stack.push(if a.gt(&b) { U256::one() } else { U256::zero() })
}

#[inline]
pub(crate) fn slt(stack: &mut Stack) {
    let a: I256 = stack.pop().into();
    let b: I256 = stack.pop().into();

    stack.push(if a.lt(&b) { U256::one() } else { U256::zero() })
}

#[inline]
pub(crate) fn sgt(stack: &mut Stack) {
    let a: I256 = stack.pop().into();
    let b: I256 = stack.pop().into();

    stack.push(if a.gt(&b) { U256::one() } else { U256::zero() })
}

#[inline]
pub(crate) fn eq(stack: &mut Stack) {
    let a = stack.pop();
    let b = stack.pop();

    stack.push(if a.eq(&b) { U256::one() } else { U256::zero() })
}

#[inline]
pub(crate) fn iszero(stack: &mut Stack) {
    let a = stack.pop();
    stack.push(if a.is_zero() {
        U256::one()
    } else {
        U256::zero()
    })
}

#[inline]
pub(crate) fn and(stack: &mut Stack) {
    let a = stack.pop();
    let b = stack.pop();
    stack.push(a & b);
}

#[inline]
pub(crate) fn or(stack: &mut Stack) {
    let a = stack.pop();
    let b = stack.pop();
    stack.push(a | b);
}

#[inline]
pub(crate) fn xor(stack: &mut Stack) {
    let a = stack.pop();
    let b = stack.pop();
    stack.push(a ^ b);
}

#[inline]
pub(crate) fn not(stack: &mut Stack) {
    let a = stack.pop();
    stack.push(!a);
}
