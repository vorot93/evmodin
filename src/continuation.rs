use crate::{host::AccessStatus, *};
use ethereum_types::*;

pub enum Interrupt {
    AccessAccount { address: Address },
    GetBalance { address: Address },
}

pub enum ResumeData {
    Dummy,
    AccessAccount { status: AccessStatus },
    GetBalance { balance: U256 },
}
