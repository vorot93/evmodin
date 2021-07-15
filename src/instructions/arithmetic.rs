use crate::{state::*, Revision, StatusCode};
use core::convert::TryInto;
use ethereum_types::{U256, U512};
use i256::I256;

pub(crate) fn add(stack: &mut Stack) {
    let a = stack.pop();
    let b = stack.pop();
    stack.push(a.overflowing_add(b).0);
}

pub(crate) fn mul(stack: &mut Stack) {
    let a = stack.pop();
    let b = stack.pop();
    stack.push(a.overflowing_mul(b).0);
}

pub(crate) fn sub(stack: &mut Stack) {
    let a = stack.pop();
    let b = stack.pop();
    stack.push(a.overflowing_sub(b).0);
}

pub(crate) fn div(stack: &mut Stack) {
    let a = stack.pop();
    let b = stack.pop();
    stack.push(if b.is_zero() { U256::zero() } else { a / b });
}

pub(crate) fn sdiv(stack: &mut Stack) {
    let a = I256::from(stack.pop());
    let b = I256::from(stack.pop());
    let v = a / b;
    stack.push(v.into());
}

pub(crate) fn modulo(stack: &mut Stack) {
    let a = stack.pop();
    let b = stack.pop();
    let v = if b.is_zero() { U256::zero() } else { a % b };
    stack.push(v);
}

pub(crate) fn smod(stack: &mut Stack) {
    let a = stack.pop();
    let b = stack.pop();

    let v = if b.is_zero() {
        U256::zero()
    } else {
        let v = I256::from(a) % I256::from(b);
        v.into()
    };

    stack.push(v);
}

pub(crate) fn addmod(stack: &mut Stack) {
    let a = U512::from(stack.pop());
    let b = U512::from(stack.pop());
    let c = U512::from(stack.pop());

    let v = if c.is_zero() {
        U256::zero()
    } else {
        let v = (a + b) % c;
        v.try_into().unwrap()
    };

    stack.push(v);
}

pub(crate) fn mulmod(stack: &mut Stack) {
    let a = U512::from(stack.pop());
    let b = U512::from(stack.pop());
    let c = U512::from(stack.pop());

    let v = if c.is_zero() {
        U256::zero()
    } else {
        let v = (a * b) % c;
        v.try_into().unwrap()
    };

    stack.push(v);
}

fn log2floor(value: U256) -> u64 {
    assert!(value != U256::zero());
    let mut l: u64 = 256;
    for i in 0..4 {
        let i = 3 - i;
        if value.0[i] == 0u64 {
            l -= 64;
        } else {
            l -= value.0[i].leading_zeros() as u64;
            if l == 0 {
                return l;
            } else {
                return l - 1;
            }
        }
    }
    l
}

pub(crate) fn exp(state: &mut ExecutionState) -> StatusCode {
    let mut base = state.stack.pop();
    let mut power = state.stack.pop();

    if !power.is_zero() {
        let additional_gas = if state.evm_revision >= Revision::Spurious {
            50
        } else {
            10
        } * (log2floor(power) / 8 + 1);

        state.gas_left -= additional_gas as i64;

        if state.gas_left < 0 {
            return StatusCode::OutOfGas;
        }
    }

    let mut v = U256::one();

    while !power.is_zero() {
        if !(power & U256::one()).is_zero() {
            v = v.overflowing_mul(base).0;
        }
        power >>= 1;
        base = base.overflowing_mul(base).0;
    }

    state.stack.push(v);

    StatusCode::Success
}

pub(crate) fn signextend(stack: &mut Stack) {
    let a = stack.pop();
    let b = stack.pop();

    let v = if a > U256::from(32) {
        b
    } else {
        let mut v = U256::zero();
        let len: usize = a.as_usize();
        let t: usize = 8 * (len + 1) - 1;
        let t_bit_mask = U256::one() << t;
        let t_value = (b & t_bit_mask) >> t;
        for i in 0..256 {
            let bit_mask = U256::one() << i;
            let i_value = (b & bit_mask) >> i;
            if i <= t {
                v = v.overflowing_add(i_value << i).0;
            } else {
                v = v.overflowing_add(t_value << i).0;
            }
        }
        v
    };

    stack.push(v);
}
