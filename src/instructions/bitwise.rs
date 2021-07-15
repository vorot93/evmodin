use crate::state::Stack;
use ethereum_types::U256;
use i256::{Sign, I256};

#[inline]
pub(crate) fn byte(stack: &mut Stack) {
    let a = stack.pop();
    let b = stack.pop();

    let mut ret = U256::zero();

    for i in 0..256 {
        if i < 8 && a < 32.into() {
            let o: usize = a.as_usize();
            let t = 255 - (7 - i + 8 * o);
            let bit_mask = U256::one() << t;
            let value = (b & bit_mask) >> t;
            ret = ret.overflowing_add(value << i).0;
        }
    }

    stack.push(ret)
}

#[inline]
pub(crate) fn shl(stack: &mut Stack) {
    let shift = stack.pop();
    let value = stack.pop();

    let ret = if value.is_zero() || shift >= U256::from(256) {
        U256::zero()
    } else {
        value << shift.as_usize()
    };

    stack.push(ret)
}

#[inline]
pub(crate) fn shr(stack: &mut Stack) {
    let shift = stack.pop();
    let value = stack.pop();

    let ret = if value.is_zero() || shift >= U256::from(256) {
        U256::zero()
    } else {
        value >> shift.as_usize()
    };

    stack.push(ret)
}

#[inline]
pub(crate) fn sar(stack: &mut Stack) {
    let shift = stack.pop();
    let value = I256::from(stack.pop());

    let ret = if value == I256::zero() || shift >= U256::from(256) {
        match value.0 {
            // value is 0 or >=1, pushing 0
            Sign::Plus | Sign::NoSign => U256::zero(),
            // value is <0, pushing -1
            Sign::Minus => I256(Sign::Minus, U256::one()).into(),
        }
    } else {
        let shift = shift.as_usize();

        match value.0 {
            Sign::Plus | Sign::NoSign => value.1 >> shift,
            Sign::Minus => {
                let shifted = ((value.1.overflowing_sub(U256::one()).0) >> shift)
                    .overflowing_add(U256::one())
                    .0;
                I256(Sign::Minus, shifted).into()
            }
        }
    };

    stack.push(ret)
}
