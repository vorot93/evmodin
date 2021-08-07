use std::str::FromStr;

use bytes::Bytes;
use ethereum_types::{Address, H256, U256};
use serde::Serialize;
use strum_macros::Display;

/// EVM revision.
#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Revision {
    /// The Frontier revision.
    /// The one Ethereum launched with.
    Frontier = 0,

    /// [The Homestead revision.](https://eips.ethereum.org/EIPS/eip-606)
    Homestead = 1,

    /// [The Tangerine Whistle revision.](https://eips.ethereum.org/EIPS/eip-608)
    Tangerine = 2,

    /// [The Spurious Dragon revision.](https://eips.ethereum.org/EIPS/eip-607)
    Spurious = 3,

    /// [The Byzantium revision.](https://eips.ethereum.org/EIPS/eip-609)
    Byzantium = 4,

    /// [The Constantinople revision.](https://eips.ethereum.org/EIPS/eip-1013)
    Constantinople = 5,

    /// [The Petersburg revision.](https://eips.ethereum.org/EIPS/eip-1716)
    Petersburg = 6,

    /// [The Istanbul revision.](https://eips.ethereum.org/EIPS/eip-1679)
    Istanbul = 7,

    /// [The Berlin revision.](https://github.com/ethereum/eth1.0-specs/blob/master/network-upgrades/mainnet-upgrades/berlin.md)
    Berlin = 8,

    /// [The London revision.](https://github.com/ethereum/eth1.0-specs/blob/master/network-upgrades/mainnet-upgrades/london.md)
    London = 9,

    /// The Shanghai revision.
    Shanghai = 10,
}

impl Revision {
    pub fn iter() -> impl IntoIterator<Item = Self> {
        [
            Self::Frontier,
            Self::Homestead,
            Self::Tangerine,
            Self::Spurious,
            Self::Byzantium,
            Self::Constantinople,
            Self::Petersburg,
            Self::Istanbul,
            Self::Berlin,
            Self::London,
            Self::Shanghai,
        ]
    }

    pub const fn latest() -> Self {
        Self::Shanghai
    }

    pub const fn len() -> usize {
        Self::latest() as usize + 1
    }
}

impl FromStr for Revision {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "frontier" => Ok(Self::Frontier),
            "homestead" => Ok(Self::Homestead),
            "tangerine" => Ok(Self::Tangerine),
            "spurious" => Ok(Self::Spurious),
            "byzantium" => Ok(Self::Byzantium),
            "constantinople" => Ok(Self::Constantinople),
            "petersburg" => Ok(Self::Petersburg),
            "istanbul" => Ok(Self::Istanbul),
            "berlin" => Ok(Self::Berlin),
            "london" => Ok(Self::London),
            "shanghai" => Ok(Self::Shanghai),
            _ => Err(()),
        }
    }
}

/// Message status code.
#[must_use]
#[derive(Clone, Debug, Display, PartialEq)]
pub enum StatusCode {
    /// Execution finished with success.
    #[strum(serialize = "success")]
    Success,

    /// Generic execution failure.
    #[strum(serialize = "failure")]
    Failure,

    /// Execution terminated with REVERT opcode.
    ///
    /// In this case the amount of gas left MAY be non-zero and additional output
    /// data MAY be provided in ::evmc_result.
    #[strum(serialize = "revert")]
    Revert,

    /// The execution has run out of gas.
    #[strum(serialize = "out of gas")]
    OutOfGas,

    /// The designated INVALID instruction has been hit during execution.
    ///
    /// [EIP-141](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-141.md)
    /// defines the instruction 0xfe as INVALID instruction to indicate execution
    /// abortion coming from high-level languages. This status code is reported
    /// in case this INVALID instruction has been encountered.
    #[strum(serialize = "invalid instruction")]
    InvalidInstruction,

    /// An undefined instruction has been encountered.
    #[strum(serialize = "undefined instruction")]
    UndefinedInstruction,

    /// The execution has attempted to put more items on the EVM stack
    /// than the specified limit.
    #[strum(serialize = "stack overflow")]
    StackOverflow,

    /// Execution of an opcode has required more items on the EVM stack.
    #[strum(serialize = "stack underflow")]
    StackUnderflow,

    /// Execution has violated the jump destination restrictions.
    #[strum(serialize = "bad jump destination")]
    BadJumpDestination,

    /// Tried to read outside memory bounds.
    ///
    /// An example is RETURNDATACOPY reading past the available buffer.
    #[strum(serialize = "invalid memory access")]
    InvalidMemoryAccess,

    /// Call depth has exceeded the limit (if any)
    #[strum(serialize = "call depth exceeded")]
    CallDepthExceeded,

    /// Tried to execute an operation which is restricted in static mode.
    #[strum(serialize = "static mode violation")]
    StaticModeViolation,

    /// A call to a precompiled or system contract has ended with a failure.
    ///
    /// An example: elliptic curve functions handed invalid EC points.
    #[strum(serialize = "precompile failure")]
    PrecompileFailure,

    /// An argument to a state accessing method has a value outside of the
    /// accepted range of values.
    #[strum(serialize = "argument out of range")]
    ArgumentOutOfRange,

    /// The caller does not have enough funds for value transfer.
    #[strum(serialize = "insufficient balance")]
    InsufficientBalance,

    /// EVM implementation generic internal error.
    #[strum(serialize = "internal error")]
    InternalError(String),
}

/// The kind of call-like instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallKind {
    Call,
    DelegateCall,
    CallCode,
    Create,
    Create2 { salt: H256 },
}

/// The message describing an EVM call,
/// including a zero-depth call from transaction origin.
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    /// The kind of the call. For zero-depth calls `CallKind::Call` SHOULD be used.
    pub kind: CallKind,

    /// Static call mode.
    pub is_static: bool,

    /// The call depth.
    pub depth: i32,

    /// The amount of gas for message execution.
    pub gas: i64,

    /// The destination of the message.
    pub destination: Address,

    /// The sender of the message.
    pub sender: Address,

    /// Message input data.
    pub input_data: Bytes,

    /// The amount of Ether transferred with the message.
    pub value: U256,
}

/// Output of EVM execution.
#[derive(Clone, Debug, PartialEq)]
pub struct Output {
    /// EVM exited with this status code.
    pub status_code: StatusCode,
    /// How much gas was left after execution
    pub gas_left: i64,
    /// Output data returned.
    pub output_data: Bytes,
    /// Contract creation address.
    pub create_address: Option<Address>,
}

/// EVM execution output if no error has occurred.
#[derive(Clone, Debug, PartialEq)]
pub struct SuccessfulOutput {
    /// Indicates if revert was requested.
    pub reverted: bool,
    /// How much gas was left after execution.
    pub gas_left: i64,
    /// Output data returned.
    pub output_data: Bytes,
}

impl From<SuccessfulOutput> for Output {
    fn from(
        SuccessfulOutput {
            reverted,
            gas_left,
            output_data,
        }: SuccessfulOutput,
    ) -> Self {
        Self {
            status_code: if reverted {
                StatusCode::Revert
            } else {
                StatusCode::Success
            },
            gas_left,
            output_data,
            create_address: None,
        }
    }
}

pub(crate) fn u256_to_address(v: U256) -> Address {
    H256(v.into()).into()
}

pub(crate) fn address_to_u256(v: Address) -> U256 {
    U256::from_big_endian(&v.0)
}
