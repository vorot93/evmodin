//! Fast EVM implementation with full async support. Port of [evmone](https://github.com/ethereum/evmone) to Rust.
//!
//! # Usage
//! ```
//! use evmodin::{*, host::*, util::*, tracing::*};
//! use ethereum_types::*;
//! use hex_literal::hex;
//!
//! let my_code = Bytecode::new()
//!     .mstore8_value(0, b'h')
//!     .mstore8_value(1, b'e')
//!     .mstore8_value(2, b'l')
//!     .mstore8_value(3, b'l')
//!     .mstore8_value(4, b'o')
//!     .ret(0, 5)
//!     .build();
//!
//! let message = Message {
//!     kind: CallKind::Call,
//!     is_static: true,
//!     depth: 0,
//!     gas: 200,
//!     destination: Address::zero(),
//!     sender: Address::zero(),
//!     input_data: vec![].into(),
//!     value: U256::zero(),
//! };
//!
//! assert_eq!(
//!     AnalyzedCode::analyze(my_code)
//!         .execute(&mut DummyHost, NoopTracer, message, Revision::latest()),
//!     Output {
//!         status_code: StatusCode::Success,
//!         gas_left: 146,
//!         output_data: b"hello".to_vec().into(),
//!         create_address: None,
//!     }
//! )
//! ```
use bytes::Bytes;
pub use common::{CallKind, Message, Output, Revision, StatusCode};
pub use host::Host;
pub use interpreter::AnalyzedCode;
pub use opcode::OpCode;

mod common;
pub mod host;
#[doc(hidden)]
pub mod instructions;
mod interpreter;
pub mod opcode;
mod state;
pub mod tracing;

mod continuation;
#[cfg(feature = "util")]
pub mod util;

#[cfg(feature = "evmc")]
pub mod evmc;
