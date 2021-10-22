# evmodin

Fast EVM implementation with support for resumability. Port of [evmone](https://github.com/ethereum/evmone) to Rust.

## Usage
```rust
use evmodin::{*, host::*, util::*, tracing::*};
use ethereum_types::*;
use hex_literal::hex;

let my_code = Bytecode::new()
    .mstore8_value(0, b'h')
    .mstore8_value(1, b'e')
    .mstore8_value(2, b'l')
    .mstore8_value(3, b'l')
    .mstore8_value(4, b'o')
    .ret(0, 5)
    .build();

let message = Message {
    kind: CallKind::Call,
    is_static: true,
    depth: 0,
    gas: 200,
    recipient: Address::zero(),
    code_address: Address::zero(),
    sender: Address::zero(),
    input_data: vec![].into(),
    value: U256::zero(),
};

assert_eq!(
    AnalyzedCode::analyze(my_code)
        .execute(&mut DummyHost, &mut NoopTracer, None, message, Revision::latest()),
    Output {
        status_code: StatusCode::Success,
        gas_left: 146,
        output_data: b"hello".to_vec().into(),
        create_address: None,
    }
)
```

## Host / interpreter separation
`evmodin` is not a standalone execution implementation - it is only an EVM interpreter with gas metering that must be coupled with Host, as defined in EVMC, for state access and inducing sub-calls. `MockedHost` is shipped in `evmodin`, but is only useful in tests.

[Akula](https://github.com/akula-bft/akula), a fully-fledged Ethereum implementation, features its own version of Host for execution. Akula+evmodin pairing is considered to be the reference execution implementation which passes all Ethereum consensus tests.

## Resumability
`evmodin` is an interpreter loop that runs until host interaction/data is necessary. Then it exits with an interrupt. Each interrupt contains a value to be supplied to the host, and `resume` method which may accept data from Host, depending on interrupt. `AnalyzedCode::execute` simply loops, using data from `Host` to resume interrupts. You can make your own reactor that will handle interrupts instead, please see `ExecutionStartInterrupt::run_to_completion_with_host` for reference implementation.
