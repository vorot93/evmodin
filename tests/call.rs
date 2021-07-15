use bytes::Bytes;
use core::iter::repeat_with;
use ethereum_types::{Address, H256, U256};
use evmodin::{opcode::*, util::*, *};
use hex_literal::hex;

#[tokio::test]
async fn delegatecall() {
    let mut value = H256::zero();
    value.0[17] = 0xfe;

    EvmTester::new()
        .code(
            Bytecode::new()
                .append(hex!("6001600003600052")) // m[0] = 0xffffff...
                .append(hex!("600560046003600260016103e8f4")) // DELEGATECALL(1000, 0x01, ...)
                .append(hex!("60086000f3")),
        )
        .apply_host_fn(|host, _| {
            host.call_result.output_data = (&hex!("0a0b0c") as &[u8]).into();
            host.call_result.gas_left = 1;
        })
        .value(value.0)
        .gas(1700)
        .gas_used(1690)
        .status(StatusCode::Success)
        .output_data(hex!("ffffffff0a0b0cff"))
        .inspect_host(move |host, _| {
            let gas_left = 1700 - 736;

            let r = host.recorded.lock();

            assert_eq!(r.calls.len(), 1);
            let call_msg = r.calls.last().unwrap();
            assert_eq!(call_msg.gas, gas_left - gas_left / 64);
            assert_eq!(call_msg.input_data.len(), 3);
            assert_eq!(<[u8; 32]>::from(call_msg.value)[17], 0xfe);
        })
        .check()
        .await
}

/// Checks if DELEGATECALL forwards the "static" flag.
#[tokio::test]
async fn delegatecall_static() {
    EvmTester::new()
        .set_static(true)
        .code(Bytecode::new().append_bc(CallInstruction::delegatecall(0).gas(1)))
        .status(StatusCode::Success)
        .gas_used(719)
        .inspect_host(|host, _| {
            let r = host.recorded.lock();

            assert_eq!(r.calls.len(), 1);
            let call_msg = r.calls.last().unwrap();
            assert_eq!(call_msg.gas, 1);
            assert!(call_msg.is_static);
        })
        .check()
        .await
}

#[tokio::test]
async fn delegatecall_oog_depth_limit() {
    let t = EvmTester::new()
        .revision(Revision::Homestead)
        .depth(1024)
        .code(
            Bytecode::new()
                .append_bc(CallInstruction::delegatecall(0).gas(16))
                .ret_top(),
        );

    t.clone()
        .status(StatusCode::Success)
        .gas_used(73)
        .output_value(0)
        .check()
        .await;

    t.clone().gas(73).status(StatusCode::OutOfGas).check().await;
}

#[tokio::test]
async fn create() {
    let address = Address::zero();

    EvmTester::new()
        .apply_host_fn(move |host, _| {
            host.accounts.entry(address).or_default().balance = 1.into();

            host.call_result.output_data = (&hex!("0a0b0c") as &[u8]).into();
            host.call_result
                .create_address
                .get_or_insert_with(Address::zero)
                .0[10] = 0xcc;
            host.call_result.gas_left = 200000;
        })
        .gas(300000)
        .code(hex!("602060006001f0600155"))
        .gas_used(115816)
        .status(StatusCode::Success)
        .inspect_host(move |host, _| {
            let mut key = H256::zero();
            key.0[31] = 1;
            assert_eq!(host.accounts[&address].storage[&key].value.0[22], 0xcc);

            let r = host.recorded.lock();
            assert_eq!(r.calls.len(), 1);
            assert_eq!(r.calls.last().unwrap().input_data.len(), 0x20);
        })
        .check()
        .await
}

#[tokio::test]
async fn create_gas() {
    for rev in [Revision::Homestead, Revision::Tangerine] {
        EvmTester::new()
            .revision(rev)
            .gas(50000)
            .code(hex!("60008080f0"))
            .status(StatusCode::Success)
            .gas_used(if rev == Revision::Homestead {
                50000
            } else {
                49719
            })
            .inspect_host(move |host, _| {
                let r = host.recorded.lock();
                assert_eq!(r.calls.len(), 1);
                assert_eq!(
                    r.calls.last().unwrap().gas,
                    if rev == Revision::Homestead {
                        17991
                    } else {
                        17710
                    }
                );
            })
            .check()
            .await
    }
}

#[tokio::test]
async fn create2() {
    let address = Address::zero();
    EvmTester::new()
        .revision(Revision::Constantinople)
        .apply_host_fn(move |host, _| {
            host.accounts.entry(address).or_default().balance = 1.into();

            host.call_result.output_data = (&hex!("0a0b0c") as &[u8]).into();
            host.call_result
                .create_address
                .get_or_insert_with(Address::zero)
                .0[10] = 0xc2;
            host.call_result.gas_left = 200000;
        })
        .gas(300000)
        .code(hex!("605a604160006001f5600155"))
        .gas_used(115817)
        .status(StatusCode::Success)
        .inspect_host(move |host, _| {
            let r = host.recorded.lock();

            assert_eq!(r.calls.len(), 1);

            let call_msg = r.calls.last().unwrap();
            assert_eq!(
                call_msg.kind,
                CallKind::Create2 {
                    salt: H256(U256::from(0x5a).into())
                }
            );
            assert_eq!(call_msg.gas, 263775);

            assert_eq!(
                host.accounts[&address].storage[&H256(U256::from(1).into())]
                    .value
                    .0[22],
                0xc2
            );

            assert_eq!(call_msg.input_data.len(), 0x41);
        })
        .check()
        .await
}

#[tokio::test]
async fn create2_salt_cost() {
    let t = EvmTester::new()
        .revision(Revision::Constantinople)
        .code(hex!("600060208180f5"));

    t.clone()
        .gas(32021)
        .status(StatusCode::Success)
        .gas_left(0)
        .inspect_host(|host, _| {
            let r = host.recorded.lock();

            assert_eq!(r.calls.len(), 1);
            assert_eq!(
                r.calls.last().unwrap().kind,
                CallKind::Create2 { salt: H256::zero() }
            );
            assert_eq!(r.calls.last().unwrap().depth, 1);
        })
        .check()
        .await;

    t.clone()
        .gas(32021 - 1)
        .status(StatusCode::OutOfGas)
        .gas_left(0)
        .inspect_host(|host, _| {
            // No another CREATE2.
            assert_eq!(host.recorded.lock().calls.len(), 0)
        })
        .check()
        .await
}

#[tokio::test]
async fn create_balance_too_low() {
    for op in [OpCode::CREATE, OpCode::CREATE2] {
        EvmTester::new()
            .revision(Revision::Constantinople)
            .apply_host_fn(|host, _| {
                host.accounts.entry(Address::zero()).or_default().balance = 1.into();
            })
            .code(
                Bytecode::new()
                    .pushv(2)
                    .opcode(OpCode::DUP1)
                    .opcode(OpCode::DUP1)
                    .opcode(OpCode::DUP1)
                    .opcode(op)
                    .ret_top(),
            )
            .status(StatusCode::Success)
            .output_value(0)
            .inspect_host(|host, _| {
                assert_eq!(host.recorded.lock().calls, []);
            })
            .check()
            .await
    }
}

#[tokio::test]
async fn create_failure() {
    for op in [OpCode::CREATE, OpCode::CREATE2] {
        let mut create_address = Address::zero();
        create_address.0[19] = 0xce;
        let t = EvmTester::new()
            .apply_host_fn(move |host, _| {
                host.call_result.create_address = Some(create_address);
            })
            .revision(Revision::Constantinople)
            .code(
                Bytecode::new()
                    .pushv(0)
                    .opcode(OpCode::DUP1)
                    .opcode(OpCode::DUP1)
                    .opcode(OpCode::DUP1)
                    .opcode(op)
                    .ret_top(),
            );

        t.clone()
            .apply_host_fn(|host, _| {
                host.call_result.status_code = StatusCode::Success;
            })
            .status(StatusCode::Success)
            .output_data(H256::from(create_address).to_fixed_bytes())
            .inspect_host(move |host, _| {
                let r = host.recorded.lock();

                assert_eq!(r.calls.len(), 1);
                assert_eq!(
                    r.calls.last().unwrap().kind,
                    if op == OpCode::CREATE {
                        CallKind::Create
                    } else {
                        CallKind::Create2 { salt: H256::zero() }
                    }
                );
            })
            .check()
            .await;

        t.clone()
            .apply_host_fn(|host, _| {
                host.call_result.status_code = StatusCode::Revert;
            })
            .status(StatusCode::Success)
            .output_value(0)
            .inspect_host(move |host, _| {
                let r = host.recorded.lock();

                assert_eq!(r.calls.len(), 1);
                assert_eq!(
                    r.calls.last().unwrap().kind,
                    if op == OpCode::CREATE {
                        CallKind::Create
                    } else {
                        CallKind::Create2 { salt: H256::zero() }
                    }
                );
            })
            .check()
            .await;

        t.clone()
            .apply_host_fn(|host, _| {
                host.call_result.status_code = StatusCode::Failure;
            })
            .status(StatusCode::Success)
            .output_value(0)
            .inspect_host(move |host, _| {
                let r = host.recorded.lock();

                assert_eq!(r.calls.len(), 1);
                assert_eq!(
                    r.calls.last().unwrap().kind,
                    if op == OpCode::CREATE {
                        CallKind::Create
                    } else {
                        CallKind::Create2 { salt: H256::zero() }
                    }
                );
            })
            .check()
            .await;
    }
}

#[tokio::test]
async fn call_failing_with_value() {
    for op in [OpCode::CALL, OpCode::CALLCODE] {
        let t = EvmTester::new()
            .apply_host_fn(|host, _| {
                host.accounts
                    .entry(hex!("00000000000000000000000000000000000000aa").into())
                    .or_default();
            })
            .code(
                Bytecode::new()
                    .pushv(0xff)
                    .pushv(0)
                    .opcode(OpCode::DUP2)
                    .opcode(OpCode::DUP2)
                    .pushv(1)
                    .pushv(0xaa)
                    .pushv(0x8000)
                    .opcode(op)
                    .opcode(OpCode::POP),
            );

        // Fails on balance check.
        t.clone()
            .gas(12000)
            .status(StatusCode::Success)
            .gas_used(7447)
            .inspect_host(|host, _| {
                // There was no call().
                assert_eq!(host.recorded.lock().calls, []);
            })
            .check()
            .await;

        // Fails on value transfer additional cost - minimum gas limit that triggers this condition.
        t.clone()
            .gas(747)
            .status(StatusCode::OutOfGas)
            .inspect_host(|host, _| {
                // There was no call().
                assert_eq!(host.recorded.lock().calls, []);
            })
            .check()
            .await;

        // Fails on value transfer additional cost - maximum gas limit that triggers this condition.
        t.clone()
            .gas(744 + 9000)
            .status(StatusCode::OutOfGas)
            .inspect_host(|host, _| {
                // There was no call().
                assert_eq!(host.recorded.lock().calls, []);
            })
            .check()
            .await;
    }
}

#[tokio::test]
async fn call_with_value() {
    let call_sender = hex!("5e4d00000000000000000000000000000000d4e5").into();
    let call_dst = hex!("00000000000000000000000000000000000000aa").into();

    EvmTester::new()
        .code(hex!("60ff600060ff6000600160aa618000f150"))
        .destination(call_sender)
        .apply_host_fn(move |host, msg| {
            host.accounts.entry(msg.destination).or_default().balance = 1.into();
            host.accounts.entry(call_dst).or_default();
            host.call_result.gas_left = 1.into();
        })
        .gas(40000)
        .gas_used(7447 + 32082)
        .status(StatusCode::Success)
        .inspect_host(move |host, _| {
            assert_eq!(
                host.recorded.lock().calls,
                [Message {
                    kind: CallKind::Call,
                    is_static: false,
                    depth: 1,
                    gas: 32083,
                    destination: call_dst,
                    sender: call_sender,
                    input_data: vec![0; 255].into(),
                    value: 1.into(),
                }]
            );
        })
        .check()
        .await
}

#[tokio::test]
async fn call_with_value_depth_limit() {
    let mut call_dst = Address::zero();
    call_dst.0[19] = 0xaa;

    EvmTester::new()
        .depth(1024)
        .apply_host_fn(move |host, _| {
            host.accounts.entry(call_dst).or_default();
        })
        .code(hex!("60ff600060ff6000600160aa618000f150"))
        .gas_used(7447)
        .status(StatusCode::Success)
        .inspect_host(|host, _| {
            assert_eq!(host.recorded.lock().calls, []);
        })
        .check()
        .await
}

#[tokio::test]
async fn call_depth_limit() {
    for op in [
        OpCode::CALL,
        OpCode::CALLCODE,
        OpCode::DELEGATECALL,
        OpCode::STATICCALL,
        OpCode::CREATE,
        OpCode::CREATE2,
    ] {
        EvmTester::new()
            .revision(Revision::Constantinople)
            .depth(1024)
            .code(
                Bytecode::new()
                    .pushv(0)
                    .opcode(OpCode::DUP1)
                    .opcode(OpCode::DUP1)
                    .opcode(OpCode::DUP1)
                    .opcode(OpCode::DUP1)
                    .opcode(OpCode::DUP1)
                    .opcode(OpCode::DUP1)
                    .opcode(op)
                    .ret_top()
                    .opcode(OpCode::INVALID),
            )
            .status(StatusCode::Success)
            .inspect_host(|host, _| {
                assert_eq!(host.recorded.lock().calls, []);
            })
            .output_value(0)
            .check()
            .await
    }
}

#[tokio::test]
async fn call_output() {
    for op in [
        OpCode::CALL,
        OpCode::CALLCODE,
        OpCode::DELEGATECALL,
        OpCode::STATICCALL,
    ] {
        let call_output = Bytes::from_static(&hex!("0a0b"));

        let t = EvmTester::new()
            .apply_host_fn({
                let call_output = call_output.clone();
                move |host, _| {
                    host.accounts.entry(Address::zero()).or_default().balance = 1.into();
                    host.call_result.output_data = call_output.clone();
                }
            })
            .inspect_host(move |host, _| {
                assert_eq!(host.call_result.output_data, call_output);
                assert!(core::ptr::eq(
                    host.call_result.output_data.as_ptr(),
                    call_output.as_ptr()
                ));
            });

        let code_prefix_output_1 = Bytecode::new()
            .pushv(1)
            .opcode(OpCode::DUP1)
            .opcode(OpCode::DUP1)
            .opcode(OpCode::DUP1)
            .opcode(OpCode::DUP1)
            .opcode(OpCode::DUP1)
            .opcode(OpCode::DUP1)
            .pushb(hex!("7fffffffffffffff"));
        let code_prefix_output_0 = Bytecode::new()
            .pushv(0)
            .opcode(OpCode::DUP1)
            .opcode(OpCode::DUP1)
            .opcode(OpCode::DUP1)
            .opcode(OpCode::DUP1)
            .opcode(OpCode::DUP1)
            .opcode(OpCode::DUP1)
            .pushb(hex!("7fffffffffffffff"));
        let code_suffix = Bytecode::new().ret(0, 3);

        t.clone()
            .code(
                Bytecode::new()
                    .append_bc(code_prefix_output_1)
                    .opcode(op)
                    .append_bc(code_suffix.clone()),
            )
            .status(StatusCode::Success)
            .output_data(hex!("000a00"))
            .check()
            .await;

        t.clone()
            .code(
                Bytecode::new()
                    .append_bc(code_prefix_output_0)
                    .opcode(op)
                    .append_bc(code_suffix.clone()),
            )
            .status(StatusCode::Success)
            .output_data(hex!("000000"))
            .check()
            .await;
    }
}

#[tokio::test]
async fn call_high_gas() {
    for call_opcode in [OpCode::CALL, OpCode::CALLCODE, OpCode::DELEGATECALL] {
        let mut call_dst = Address::zero();
        call_dst.0[19] = 0xaa;

        EvmTester::new()
            .revision(Revision::Homestead)
            .apply_host_fn(move |host, _| {
                host.accounts.entry(call_dst).or_default();
            })
            .gas(5000)
            .code(
                Bytecode::new()
                    .pushv(0)
                    .pushv(0)
                    .pushv(0)
                    .pushv(0)
                    .pushv(0)
                    .pushv(0xaa)
                    .pushv(0x134c)
                    .opcode(call_opcode),
            )
            .status(StatusCode::OutOfGas)
            .check()
            .await
    }
}

#[tokio::test]
async fn call_value_zero_to_nonexistent_account() {
    let call_gas = 6000;

    let gas_left = 1000;

    EvmTester::new()
        .apply_host_fn(move |host, _| {
            host.call_result.gas_left = gas_left;
        })
        .code(
            Bytecode::new()
                .pushv(0x40)
                .pushv(0)
                .pushv(0x40)
                .pushv(0)
                .pushv(0)
                .pushv(0xaa)
                .pushv(call_gas)
                .opcode(OpCode::CALL)
                .opcode(OpCode::POP),
        )
        .gas(9000)
        .gas_used(729 + (call_gas - gas_left))
        .status(StatusCode::Success)
        .inspect_host(|host, _| {
            assert_eq!(
                host.recorded.lock().calls,
                [Message {
                    kind: CallKind::Call,
                    is_static: false,
                    depth: 1,
                    gas: 6000,
                    destination: hex!("00000000000000000000000000000000000000aa").into(),
                    sender: Address::zero(),
                    input_data: vec![0; 64].into(),
                    value: 0.into(),
                }]
            );
        })
        .check()
        .await
}

#[tokio::test]
async fn call_new_account_creation_cost() {
    let call_dst: Address = hex!("00000000000000000000000000000000000000ad").into();
    let destination: Address = hex!("0000000000000000000000000000000000000003").into();

    let t = EvmTester::new()
        .code(
            Bytecode::new()
                .pushv(0)
                .pushv(0)
                .pushv(0)
                .pushv(0)
                .pushv(0)
                .opcode(OpCode::CALLDATALOAD)
                .pushb(call_dst.0)
                .pushv(0)
                .opcode(OpCode::CALL)
                .ret_top(),
        )
        .destination(destination);

    t.clone()
        .revision(Revision::Tangerine)
        .apply_host_fn(|host, msg| {
            host.accounts.entry(msg.destination).or_default().balance = 0.into();
        })
        .input(&hex!("00") as &[u8])
        .status(StatusCode::Success)
        .gas_used(25000 + 739)
        .output_value(1)
        .inspect_host(move |host, _| {
            assert_eq!(
                host.recorded.lock().account_accesses,
                [
                    call_dst, // Account exist?
                    call_dst, // Call.
                ]
            );
        })
        .check()
        .await;

    t.clone()
        .revision(Revision::Tangerine)
        .apply_host_fn(|host, msg| {
            host.accounts.entry(msg.destination).or_default().balance = 1.into();
        })
        .input(&hex!("0000000000000000000000000000000000000000000000000000000000000001") as &[u8])
        .status(StatusCode::Success)
        .gas_used(25000 + 9000 + 739)
        .output_value(1)
        .inspect_host(move |host, msg| {
            assert_eq!(
                host.recorded.lock().calls,
                [Message {
                    kind: CallKind::Call,
                    is_static: false,
                    depth: 1,
                    gas: 2300,
                    destination: call_dst,
                    sender: destination,
                    input_data: vec![].into(),
                    value: 1.into(),
                }]
            );
            assert_eq!(
                host.recorded.lock().account_accesses,
                [
                    call_dst,        // Account exist?
                    msg.destination, // Balance.
                    call_dst         // Call.
                ]
            )
        })
        .check()
        .await;

    t.clone()
        .revision(Revision::Spurious)
        .apply_host_fn(|host, msg| {
            host.accounts.entry(msg.destination).or_default().balance = 0.into();
        })
        .input(&hex!("00") as &[u8])
        .status(StatusCode::Success)
        .gas_used(739)
        .output_value(1)
        .inspect_host(move |host, _| {
            assert_eq!(
                host.recorded.lock().calls,
                [Message {
                    kind: CallKind::Call,
                    is_static: false,
                    depth: 1,
                    gas: 0,
                    destination: call_dst,
                    sender: destination,
                    input_data: vec![].into(),
                    value: 0.into(),
                }]
            );
            assert_eq!(
                host.recorded.lock().account_accesses,
                [
                call_dst         // Call.
            ]
            )
        })
        .check()
        .await;

    t.clone()
        .revision(Revision::Spurious)
        .apply_host_fn(|host, msg| {
            host.accounts.entry(msg.destination).or_default().balance = 1.into();
        })
        .input(&hex!("0000000000000000000000000000000000000000000000000000000000000001") as &[u8])
        .status(StatusCode::Success)
        .gas_used(25000 + 9000 + 739)
        .output_value(1)
        .inspect_host(move |host, msg| {
            assert_eq!(
                host.recorded.lock().calls,
                [Message {
                    kind: CallKind::Call,
                    is_static: false,
                    depth: 1,
                    gas: 2300,
                    destination: call_dst,
                    sender: destination,
                    input_data: vec![].into(),
                    value: 1.into(),
                }]
            );
            assert_eq!(
                host.recorded.lock().account_accesses,
                [
                    call_dst,        // Account exist?
                    msg.destination, // Balance.
                    call_dst         // Call.
                ]
            )
        })
        .check()
        .await
}

#[tokio::test]
async fn callcode_new_account_create() {
    let code = hex!("60008080806001600061c350f250");
    let call_sender = hex!("5e4d00000000000000000000000000000000d4e5").into();

    EvmTester::new()
        .destination(call_sender)
        .apply_host_fn(|host, msg| {
            host.accounts.entry(msg.destination).or_default().balance = 1.into();
            host.call_result.gas_left = 1;
        })
        .gas(100000)
        .code(code)
        .gas_used(59722)
        .status(StatusCode::Success)
        .inspect_host(move |host, _| {
            assert_eq!(
                host.recorded.lock().calls,
                [Message {
                    kind: CallKind::CallCode,
                    is_static: false,
                    depth: 1,
                    gas: 52300,
                    destination: Address::zero(),
                    sender: call_sender,
                    input_data: vec![].into(),
                    value: 1.into(),
                }]
            );
        })
        .check()
        .await
}

/// Performs a CALL then OOG in the same code block.
#[tokio::test]
async fn call_then_oog() {
    let call_dst = 0xaa;

    let mut code = Bytecode::new().append_bc(
        CallInstruction::call(call_dst)
            .gas(254)
            .value(0)
            .input(0, 0x40)
            .output(0, 0x40),
    );

    for _ in 0..4 {
        code = code.opcode(OpCode::DUP1).opcode(OpCode::ADD);
    }

    code = code.opcode(OpCode::POP);

    EvmTester::new()
        .apply_host_fn(move |host, _| {
            let mut address = Address::zero();
            address.0[19] = call_dst;
            host.accounts.entry(address).or_default();

            host.call_result.status_code = StatusCode::Failure;
            host.call_result.gas_left = 0;
        })
        .code(code)
        .gas(1000)
        .gas_used(1000)
        .gas_left(0)
        .status(StatusCode::OutOfGas)
        .inspect_host(|host, _| {
            assert_eq!(host.recorded.lock().calls.len(), 1);
            assert_eq!(host.recorded.lock().calls[0].gas, 254);
        })
        .check()
        .await
}

/// Performs a CALLCODE then OOG in the same code block.
#[tokio::test]
async fn callcode_then_oog() {
    let call_dst = 0xaa;

    let mut code = Bytecode::new().append_bc(
        CallInstruction::callcode(call_dst)
            .gas(100)
            .value(0)
            .input(0, 3)
            .output(3, 9),
    );

    for _ in 0..4 {
        code = code.opcode(OpCode::DUP1).opcode(OpCode::ADD);
    }

    code = code.opcode(OpCode::POP);

    EvmTester::new()
        .apply_host_fn(move |host, _| {
            let mut address = Address::zero();
            address.0[19] = call_dst;
            host.accounts.entry(address).or_default();

            host.call_result.status_code = StatusCode::Failure;
            host.call_result.gas_left = 0;
        })
        .code(code)
        .gas(825)
        .status(StatusCode::OutOfGas)
        .inspect_host(|host, _| {
            assert_eq!(host.recorded.lock().calls.len(), 1);
            assert_eq!(host.recorded.lock().calls[0].gas, 100);
        })
        .check()
        .await
}

/// Performs a CALL then OOG in the same code block.
#[tokio::test]
async fn delegatecall_then_oog() {
    let call_dst = 0xaa;

    let mut code = Bytecode::new().append_bc(
        CallInstruction::delegatecall(call_dst)
            .gas(254)
            .input(0, 64)
            .output(0, 64),
    );

    for _ in 0..4 {
        code = code.opcode(OpCode::DUP1).opcode(OpCode::ADD);
    }

    code = code.opcode(OpCode::POP);

    EvmTester::new()
        .apply_host_fn(move |host, _| {
            let mut address = Address::zero();
            address.0[19] = call_dst;
            host.accounts.entry(address).or_default();

            host.call_result.status_code = StatusCode::Failure;
            host.call_result.gas_left = 0;
        })
        .code(code)
        .gas(1000)
        .gas_used(1000)
        .gas_left(0)
        .status(StatusCode::OutOfGas)
        .inspect_host(|host, _| {
            assert_eq!(host.recorded.lock().calls.len(), 1);
            assert_eq!(host.recorded.lock().calls[0].gas, 254);
        })
        .check()
        .await
}

/// Performs a STATICCALL then OOG in the same code block.
#[tokio::test]
async fn staticcall_then_oog() {
    let call_dst = 0xaa;

    let mut code = Bytecode::new().append_bc(
        CallInstruction::staticcall(call_dst)
            .gas(254)
            .input(0, 0x40)
            .output(0, 0x40),
    );

    for _ in 0..4 {
        code = code.opcode(OpCode::DUP1).opcode(OpCode::ADD);
    }

    code = code.opcode(OpCode::POP);

    EvmTester::new()
        .apply_host_fn(move |host, _| {
            let mut address = Address::zero();
            address.0[19] = call_dst;
            host.accounts.entry(address).or_default();

            host.call_result.status_code = StatusCode::Failure;
            host.call_result.gas_left = 0;
        })
        .code(code)
        .gas(1000)
        .status(StatusCode::OutOfGas)
        .gas_used(1000)
        .gas_left(0)
        .inspect_host(|host, _| {
            assert_eq!(host.recorded.lock().calls.len(), 1);
            assert_eq!(host.recorded.lock().calls[0].gas, 254);
        })
        .check()
        .await
}

#[tokio::test]
async fn staticcall_input() {
    EvmTester::new()
        .code(
            Bytecode::new()
                .mstore_value(3, 0x010203)
                .append_bc(CallInstruction::staticcall(0).gas(0xee).input(32, 3)),
        )
        .inspect_host(|host, _| {
            let r = host.recorded.lock();

            assert_eq!(r.calls.len(), 1);
            assert_eq!(r.calls[0].gas, 0xee);
            assert_eq!(r.calls[0].input_data[..], hex!("010203"));
        })
        .check()
        .await
}

#[tokio::test]
async fn call_with_value_low_gas() {
    for op in [OpCode::CALL, OpCode::CALLCODE] {
        EvmTester::new()
            .apply_host_fn(|host, _| {
                // Create the call destination account.
                host.accounts.entry(Address::zero()).or_default();
            })
            .code(
                Bytecode::new()
                    .pushv(0)
                    .pushv(0)
                    .pushv(0)
                    .pushv(0)
                    .pushv(1)
                    .pushv(0)
                    .pushv(0)
                    .opcode(op)
                    .opcode(OpCode::POP),
            )
            .gas(9721)
            .status(StatusCode::Success)
            .gas_left(2300 - 2)
            .check()
            .await
    }
}

#[tokio::test]
async fn call_oog_after_balance_check() {
    // Create the call destination account.
    for op in [OpCode::CALL, OpCode::CALLCODE] {
        EvmTester::new()
            .apply_host_fn(|host, _| {
                // Create the call destination account.
                host.accounts.entry(Address::zero()).or_default();
            })
            .code(
                Bytecode::new()
                    .pushv(0)
                    .pushv(0)
                    .pushv(0)
                    .pushv(0)
                    .pushv(1)
                    .pushv(0)
                    .pushv(0)
                    .opcode(op)
                    .opcode(OpCode::SELFDESTRUCT),
            )
            .gas(12420)
            .status(StatusCode::OutOfGas)
            .check()
            .await
    }
}

#[tokio::test]
async fn call_oog_after_depth_check() {
    // Create the call destination account.
    let t = EvmTester::new()
        .apply_host_fn(|host, _| {
            host.accounts.entry(Address::zero()).or_default();
        })
        .depth(1024);

    for op in [OpCode::CALL, OpCode::CALLCODE] {
        t.clone()
            .code(
                Bytecode::new()
                    .pushv(0)
                    .pushv(0)
                    .pushv(0)
                    .pushv(0)
                    .pushv(1)
                    .pushv(0)
                    .pushv(0)
                    .opcode(op)
                    .opcode(OpCode::SELFDESTRUCT),
            )
            .gas(12420)
            .status(StatusCode::OutOfGas)
            .check()
            .await
    }

    let t = t.clone().revision(Revision::Tangerine).code(
        Bytecode::new()
            .pushv(0)
            .pushv(0)
            .pushv(0)
            .pushv(0)
            .pushv(0)
            .pushv(0)
            .pushv(0)
            .opcode(OpCode::CALL)
            .opcode(OpCode::SELFDESTRUCT),
    );

    t.clone()
        .gas(721)
        .status(StatusCode::OutOfGas)
        .check()
        .await;

    t.clone()
        .gas(721 + 5000 - 1)
        .status(StatusCode::OutOfGas)
        .check()
        .await;
}

#[tokio::test]
async fn create_oog_after() {
    for op in [OpCode::CREATE, OpCode::CREATE2] {
        EvmTester::new()
            .revision(Revision::Constantinople)
            .code(
                Bytecode::new()
                    .pushv(0)
                    .pushv(0)
                    .pushv(0)
                    .pushv(0)
                    .opcode(op)
                    .opcode(OpCode::SELFDESTRUCT),
            )
            .gas(39000)
            .status(StatusCode::OutOfGas)
            .check()
            .await
    }
}

#[tokio::test]
async fn returndatasize_before_call() {
    EvmTester::new()
        .code(hex!("3d60005360016000f3"))
        .gas_used(17)
        .output_data([0])
        .check()
        .await
}

#[tokio::test]
async fn returndatasize() {
    let call_res_output_len = 13;

    let t = EvmTester::new()
        .apply_host_fn(move |host, _| {
            host.call_result.output_data = repeat_with(rand::random)
                .take(call_res_output_len as usize)
                .collect::<Vec<u8>>()
                .into()
        })
        .code(
            Bytecode::new()
                .pushv(0)
                .opcode(OpCode::DUP1)
                .opcode(OpCode::DUP1)
                .opcode(OpCode::DUP1)
                .opcode(OpCode::DUP1)
                .opcode(OpCode::DUP1)
                .opcode(OpCode::DELEGATECALL)
                .opcode(OpCode::RETURNDATASIZE)
                .mstore8(0)
                .pushv(1)
                .pushv(0)
                .opcode(OpCode::RETURN),
        );

    t.clone()
        .gas_used(735)
        .output_data([call_res_output_len])
        .check()
        .await;

    t.clone()
        .apply_host_fn(|host, _| {
            host.call_result.output_data = vec![0; 1].into();
            host.call_result.status_code = StatusCode::Failure;
        })
        .gas_used(735)
        .output_data([1])
        .check()
        .await;

    t.clone()
        .apply_host_fn(|host, _| {
            host.call_result.output_data = Bytes::new();
            host.call_result.status_code = StatusCode::InternalError;
        })
        .gas_used(735)
        .output_data([0])
        .check()
        .await;
}

#[tokio::test]
async fn returndatacopy() {
    let call_output = hex!("0102030405060700000000000000000000000000000000000000000000000000");

    EvmTester::new()
        .apply_host_fn(move |host, _| {
            host.call_result.output_data = Bytes::from(call_output.to_vec());
        })
        .code(hex!("600080808060aa60fff4506020600060003e60206000f3"))
        .gas_used(999)
        .output_data(call_output)
        .check()
        .await
}

#[tokio::test]
async fn returndatacopy_empty() {
    EvmTester::new()
        .code(hex!("600080808060aa60fff4600080803e60016000f3"))
        .gas_used(994)
        .output_data([0])
        .check()
        .await
}

#[tokio::test]
async fn returndatacopy_cost() {
    let t = EvmTester::new()
        .code(hex!("60008080808080fa6001600060003e"))
        .apply_host_fn(|host, _| {
            host.call_result.output_data = vec![0].into();
        });
    t.clone().gas(736).status(StatusCode::Success).check().await;
    t.clone()
        .gas(735)
        .status(StatusCode::OutOfGas)
        .check()
        .await;
}

#[tokio::test]
async fn returndatacopy_outofrange() {
    for code in [
        hex!("60008080808080fa6002600060003e"),
        hex!("60008080808080fa6001600160003e"),
        hex!("60008080808080fa6000600260003e"),
    ] {
        EvmTester::new()
            .apply_host_fn(|host, _| {
                host.call_result.output_data = vec![0].into();
            })
            .code(code)
            .gas(735)
            .status(StatusCode::InvalidMemoryAccess)
            .check()
            .await
    }
}
