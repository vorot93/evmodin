use crate::{
    common::{Revision, StatusCode},
    state::ExecutionState,
};
use ethereum_types::U256;

pub(crate) mod arithmetic;
pub(crate) mod bitwise;
pub(crate) mod boolean;
pub(crate) mod call;
pub(crate) mod control;
pub(crate) mod external;
pub(crate) mod instruction_table;
pub(crate) mod memory;
pub(crate) mod properties;
pub(crate) mod stack_manip;

#[must_use]
pub(crate) enum InstructionResolution {
    Continue,
    Exit(StatusCode),
    Jump(usize),
}

pub use properties::PROPERTIES;
