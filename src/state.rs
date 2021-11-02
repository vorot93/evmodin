use crate::common::{Message, Revision};
use alloc::vec::Vec;
use arrayvec::ArrayVec;
use bytes::Bytes;
use ethereum_types::U256;
use getset::{Getters, MutGetters};

const SIZE: usize = 1024;

/// EVM stack.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Stack(pub ArrayVec<U256, SIZE>);

impl Stack {
    pub const fn limit() -> usize {
        SIZE
    }

    fn get_pos(&self, pos: usize) -> usize {
        self.len() - 1 - pos
    }

    pub fn get(&self, pos: usize) -> &U256 {
        &self.0[self.get_pos(pos)]
    }

    pub fn get_mut(&mut self, pos: usize) -> &mut U256 {
        let pos = self.get_pos(pos);
        &mut self.0[pos]
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&mut self, v: U256) {
        unsafe { self.0.push_unchecked(v) }
    }

    pub fn pop(&mut self) -> U256 {
        self.0.pop().expect("underflow")
    }

    pub fn swap_top(&mut self, pos: usize) {
        let top = self.0.len() - 1;
        let pos = self.get_pos(pos);
        self.0.swap(top, pos);
    }
}

pub type Memory = Vec<u8>;

/// EVM execution state.
#[derive(Clone, Debug, Getters, MutGetters)]
pub struct ExecutionState {
    #[getset(get = "pub", get_mut = "pub")]
    pub(crate) gas_left: i64,
    #[getset(get = "pub", get_mut = "pub")]
    pub(crate) stack: Stack,
    #[getset(get = "pub", get_mut = "pub")]
    pub(crate) memory: Memory,
    pub(crate) message: Message,
    pub(crate) evm_revision: Revision,
    #[getset(get = "pub", get_mut = "pub")]
    pub(crate) return_data: Bytes,
    pub(crate) output_data: Bytes,
}

impl ExecutionState {
    pub fn new(message: Message, evm_revision: Revision) -> Self {
        Self {
            gas_left: message.gas,
            stack: Default::default(),
            memory: Memory::with_capacity(4 * 1024),
            message,
            evm_revision,
            return_data: Default::default(),
            output_data: Bytes::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack() {
        let mut stack = Stack::default();

        let items = [0xde, 0xad, 0xbe, 0xef];

        for (i, item) in items.iter().copied().enumerate() {
            stack.push(item.into());
            assert_eq!(stack.len(), i + 1);
        }

        assert_eq!(*stack.get(2), 0xad.into());

        assert_eq!(stack.pop(), 0xef.into());

        assert_eq!(*stack.get(2), 0xde.into());
    }
}
