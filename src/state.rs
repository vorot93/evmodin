use crate::common::{Message, Revision};
use arrayvec::ArrayVec;
use bytes::Bytes;
use ethereum_types::U256;
use serde::Serialize;

const SIZE: usize = 1024;

#[derive(Clone, Debug, Default, Serialize)]
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

    pub fn push(&mut self, v: U256) {
        self.0.push(v)
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

/// Execution state
#[derive(Debug)]
pub struct ExecutionState {
    pub(crate) gas_left: i64,
    pub(crate) stack: Stack,
    pub(crate) memory: Memory,
    pub(crate) message: Message,
    pub(crate) evm_revision: Revision,
    pub(crate) return_data: Bytes,
    pub(crate) output_data: Bytes,
    pub(crate) current_block_cost: u32,
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
            current_block_cost: 0,
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
