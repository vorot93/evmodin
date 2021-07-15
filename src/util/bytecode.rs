use crate::opcode::*;
use core::iter::repeat;
use ethereum_types::U256;
use std::ops::{Add, Mul};

/// EVM bytecode builder.
#[derive(Clone, Debug, PartialEq)]
pub struct Bytecode {
    inner: Vec<u8>,
}

impl Bytecode {
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn append(mut self, b: impl IntoIterator<Item = u8>) -> Self {
        self.inner.append(&mut b.into_iter().collect::<Vec<_>>());
        self
    }

    pub fn append_bc(mut self, b: impl Into<Self>) -> Self {
        self.inner.append(&mut b.into().build());
        self
    }

    pub fn repeat(mut self, n: usize) -> Self {
        self.inner = repeat(self.inner.into_iter()).take(n).flatten().collect();
        self
    }

    pub fn pushv(self, value: impl Into<U256>) -> Self {
        let value = value.into();
        let b = <[u8; 32]>::from(value)
            .iter()
            .skip_while(|&&v| v == 0)
            .copied()
            .collect::<Vec<_>>();

        self.pushb(b)
    }

    pub fn pushb(mut self, b: impl IntoIterator<Item = u8>) -> Self {
        let mut b = b.into_iter().collect::<Vec<_>>();

        if b.is_empty() {
            b.push(0);
        }

        self.inner
            .extend_from_slice(&[(b.len() + OpCode::PUSH1.to_usize() - 1) as u8]);
        self.inner.append(&mut b);

        self
    }

    pub fn opcode(mut self, opcode: OpCode) -> Self {
        self.inner.push(opcode.to_u8());
        self
    }

    pub fn ret(mut self, index: impl Into<U256>, size: impl Into<U256>) -> Self {
        self = self.pushv(size);
        self = self.pushv(index);
        self = self.opcode(OpCode::RETURN);
        self
    }

    pub fn mstore(mut self, index: impl Into<U256>) -> Self {
        self = self.pushv(index);
        self = self.opcode(OpCode::MSTORE);
        self
    }

    pub fn mstore_value(mut self, index: impl Into<U256>, value: impl Into<U256>) -> Self {
        self = self.pushv(value);
        self = self.pushv(index);
        self = self.opcode(OpCode::MSTORE);
        self
    }

    pub fn mstore8(mut self, index: impl Into<U256>) -> Self {
        self = self.pushv(index);
        self = self.opcode(OpCode::MSTORE8);
        self
    }

    pub fn mstore8_value(mut self, index: impl Into<U256>, value: impl Into<U256>) -> Self {
        self = self.pushv(value);
        self = self.pushv(index);
        self = self.opcode(OpCode::MSTORE8);
        self
    }

    pub fn ret_top(self) -> Self {
        self.mstore(0).ret(0, 0x20)
    }

    pub fn jump(self, target: impl Into<U256>) -> Self {
        self.pushv(target).opcode(OpCode::JUMP)
    }

    pub fn jumpi(self, target: impl Into<Bytecode>, condition: impl Into<Bytecode>) -> Self {
        self.append(condition.into().build())
            .append(target.into().build())
            .opcode(OpCode::JUMPI)
    }

    pub fn sstore(self, index: impl Into<U256>, value: impl Into<U256>) -> Self {
        self.pushv(value).pushv(index).opcode(OpCode::SSTORE)
    }

    pub fn sload(self, index: impl Into<U256>) -> Self {
        self.pushv(index).opcode(OpCode::SLOAD)
    }

    pub fn build(self) -> Vec<u8> {
        self.inner
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl From<U256> for Bytecode {
    fn from(value: U256) -> Self {
        Self::new().pushv(value)
    }
}

impl From<OpCode> for Bytecode {
    fn from(opcode: OpCode) -> Self {
        Self::new().opcode(opcode)
    }
}

impl<const N: usize> From<[u8; N]> for Bytecode {
    fn from(inner: [u8; N]) -> Self {
        Self {
            inner: Vec::from(&inner as &[u8]),
        }
    }
}

impl From<Vec<u8>> for Bytecode {
    fn from(inner: Vec<u8>) -> Self {
        Self { inner }
    }
}

impl AsRef<[u8]> for Bytecode {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

impl IntoIterator for Bytecode {
    type Item = u8;
    type IntoIter = <Vec<u8> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl Mul<Bytecode> for usize {
    type Output = Bytecode;

    fn mul(self, rhs: Bytecode) -> Self::Output {
        repeat(rhs)
            .take(self)
            .fold(Bytecode::new(), |acc, b| acc.append_bc(b))
    }
}

impl Mul<OpCode> for usize {
    type Output = Bytecode;

    fn mul(self, rhs: OpCode) -> Self::Output {
        self.mul(Bytecode::from(rhs))
    }
}

impl<T: Into<Bytecode>> Add<T> for Bytecode {
    type Output = Bytecode;

    fn add(self, rhs: T) -> Self::Output {
        self.append_bc(rhs)
    }
}

pub struct CallInstruction {
    op: OpCode,
    address: U256,
    gas: U256,
    value: U256,
    input: U256,
    input_size: U256,
    output: U256,
    output_size: U256,
}

impl CallInstruction {
    fn new(op: OpCode, address: impl Into<U256>) -> Self {
        Self {
            op,
            address: address.into(),
            gas: 0.into(),
            value: 0.into(),
            input: 0.into(),
            input_size: 0.into(),
            output: 0.into(),
            output_size: 0.into(),
        }
    }

    pub fn delegatecall(address: impl Into<U256>) -> Self {
        Self::new(OpCode::DELEGATECALL, address)
    }

    pub fn staticcall(address: impl Into<U256>) -> Self {
        Self::new(OpCode::STATICCALL, address)
    }

    pub fn call(address: impl Into<U256>) -> Self {
        Self::new(OpCode::CALL, address)
    }

    pub fn callcode(address: impl Into<U256>) -> Self {
        Self::new(OpCode::CALLCODE, address)
    }

    pub fn opcode(&self) -> OpCode {
        self.op
    }

    pub fn gas(mut self, gas: impl Into<U256>) -> Self {
        self.gas = gas.into();
        self
    }

    pub fn value(mut self, value: impl Into<U256>) -> Self {
        self.value = value.into();
        self
    }

    pub fn input(mut self, index: impl Into<U256>, size: impl Into<U256>) -> Self {
        self.input = index.into();
        self.input_size = size.into();
        self
    }

    pub fn output(mut self, index: impl Into<U256>, size: impl Into<U256>) -> Self {
        self.output = index.into();
        self.output_size = size.into();
        self
    }
}

impl From<CallInstruction> for Bytecode {
    fn from(call: CallInstruction) -> Self {
        let mut b = Bytecode::new()
            .pushv(call.output_size)
            .pushv(call.output)
            .pushv(call.input_size)
            .pushv(call.input);
        if call.op == OpCode::CALL || call.op == OpCode::CALLCODE {
            b = b.pushv(call.value);
        }
        b.pushv(call.address).pushv(call.gas).opcode(call.op)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiply_bytecode() {
        assert_eq!(
            3 * Bytecode::new().opcode(OpCode::POP),
            Bytecode::new()
                .opcode(OpCode::POP)
                .opcode(OpCode::POP)
                .opcode(OpCode::POP)
        )
    }
}
