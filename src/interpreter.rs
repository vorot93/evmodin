use self::instruction_table::*;
use crate::{
    common::*,
    continuation::{interrupt::*, interrupt_data::*, resume_data::*, *},
    instructions::{control::*, stack_manip::*, *},
    state::*,
    tracing::Tracer,
    *,
};
use ethereum_types::{Address, U256};
use genawaiter::sync::*;

fn check_requirements(
    instruction_table: &InstructionTable,
    state: &mut ExecutionState,
    op: OpCode,
) -> Result<(), StatusCode> {
    let metrics = if let Some(v) = instruction_table[op.to_usize()] {
        v
    } else {
        return Err(StatusCode::UndefinedInstruction);
    };

    state.gas_left -= metrics.gas_cost as i64;
    if state.gas_left < 0 {
        return Err(StatusCode::OutOfGas);
    }

    let stack_size = state.stack.len();
    if stack_size == Stack::limit() {
        if metrics.can_overflow_stack {
            return Err(StatusCode::StackOverflow);
        }
    } else if stack_size < metrics.stack_height_required.into() {
        return Err(StatusCode::StackUnderflow);
    }

    Ok(())
}

#[derive(Clone, Debug)]
pub struct JumpdestMap(Vec<bool>);

impl JumpdestMap {
    pub fn contains(&self, dst: U256) -> bool {
        dst < self.0.len().into() && self.0[dst.as_usize()]
    }
}

/// Code with analysis.
#[derive(Clone, Debug)]
pub struct AnalyzedCode {
    jumpdest_map: JumpdestMap,
    code: Bytes,
}

impl AnalyzedCode {
    /// Analyze code and prepare it for execution.
    pub fn analyze(code: impl AsRef<[u8]>) -> Self {
        let code = code.as_ref();
        let mut jumpdest_map = vec![false; code.len()];

        let mut i = 0;
        while i < code.len() {
            let opcode = OpCode(code[i]);
            i += match opcode {
                OpCode::JUMPDEST => {
                    jumpdest_map[i] = true;
                    1
                }
                OpCode::PUSH1
                | OpCode::PUSH2
                | OpCode::PUSH3
                | OpCode::PUSH4
                | OpCode::PUSH5
                | OpCode::PUSH6
                | OpCode::PUSH7
                | OpCode::PUSH8
                | OpCode::PUSH9
                | OpCode::PUSH10
                | OpCode::PUSH11
                | OpCode::PUSH12
                | OpCode::PUSH13
                | OpCode::PUSH14
                | OpCode::PUSH15
                | OpCode::PUSH16
                | OpCode::PUSH17
                | OpCode::PUSH18
                | OpCode::PUSH19
                | OpCode::PUSH20
                | OpCode::PUSH21
                | OpCode::PUSH22
                | OpCode::PUSH23
                | OpCode::PUSH24
                | OpCode::PUSH25
                | OpCode::PUSH26
                | OpCode::PUSH27
                | OpCode::PUSH28
                | OpCode::PUSH29
                | OpCode::PUSH30
                | OpCode::PUSH31
                | OpCode::PUSH32 => opcode.to_usize() - OpCode::PUSH1.to_usize() + 2,
                _ => 1,
            }
        }

        let mut padded_code = vec![0_u8; i + 1];
        padded_code[..code.len()].copy_from_slice(code);
        padded_code[i] = OpCode::STOP.to_u8();

        let jumpdest_map = JumpdestMap(jumpdest_map);
        let code = padded_code.into();

        Self { jumpdest_map, code }
    }

    /// Execute analyzed EVM bytecode.
    pub fn execute<H: Host, T: Tracer + 'static>(
        &self,
        host: &mut H,
        mut tracer: T,
        message: Message,
        revision: Revision,
    ) -> Output {
        if !T::DUMMY {
            tracer.notify_execution_start(revision, message.clone(), self.code.clone());
        }

        let mut interrupt =
            InterruptVariant::ExecutionStart(self.execute_resumable(!T::DUMMY, message, revision));

        loop {
            interrupt = match interrupt {
                InterruptVariant::ExecutionStart(i) => i.resume(()),
                InterruptVariant::InstructionStart(i) => {
                    tracer.notify_instruction_start(i.data().pc, i.data().opcode, &i.data().state);
                    i.resume(())
                }
                InterruptVariant::AccountExists(i) => {
                    let exists = host.account_exists(i.data().address);
                    i.resume(AccountExistsStatus { exists })
                }
                InterruptVariant::GetBalance(i) => {
                    let balance = host.get_balance(i.data().address);
                    i.resume(Balance { balance })
                }
                InterruptVariant::GetCodeSize(i) => {
                    let code_size = host.get_code_size(i.data().address);
                    i.resume(CodeSize { code_size })
                }
                InterruptVariant::GetStorage(i) => {
                    let value = host.get_storage(i.data().address, i.data().key);
                    i.resume(StorageValue { value })
                }
                InterruptVariant::SetStorage(i) => {
                    let status = host.set_storage(i.data().address, i.data().key, i.data().value);
                    i.resume(StorageStatusInfo { status })
                }
                InterruptVariant::GetCodeHash(i) => {
                    let hash = host.get_code_hash(i.data().address);
                    i.resume(CodeHash { hash })
                }
                InterruptVariant::CopyCode(i) => {
                    let mut code = vec![0; i.data().max_size];
                    let copied = host.copy_code(i.data().address, i.data().offset, &mut code[..]);
                    if copied > code.len() {
                        return Output {
                            status_code: StatusCode::InternalError(format!(
                                "copy code: copied {} > max size {}",
                                copied,
                                code.len()
                            )),
                            gas_left: 0,
                            output_data: Bytes::new(),
                            create_address: None,
                        };
                    }
                    code.truncate(copied);
                    let code = code.into();
                    i.resume(Code { code })
                }
                InterruptVariant::Selfdestruct(i) => {
                    host.selfdestruct(i.data().address, i.data().beneficiary);
                    i.resume(())
                }
                InterruptVariant::Call(i) => {
                    let output = host.call(&i.data().message);
                    i.resume(CallOutput { output })
                }
                InterruptVariant::GetTxContext(i) => {
                    let context = host.get_tx_context();
                    i.resume(TxContextData { context })
                }
                InterruptVariant::GetBlockHash(i) => {
                    let hash = host.get_block_hash(i.data().block_number);
                    i.resume(BlockHash { hash })
                }
                InterruptVariant::EmitLog(i) => {
                    host.emit_log(
                        i.data().address,
                        &*i.data().data,
                        i.data().topics.as_slice(),
                    );
                    i.resume(())
                }
                InterruptVariant::AccessAccount(i) => {
                    let status = host.access_account(i.data().address);
                    i.resume(AccessAccountStatus { status })
                }
                InterruptVariant::AccessStorage(i) => {
                    let status = host.access_storage(i.data().address, i.data().key);
                    i.resume(AccessStorageStatus { status })
                }
                InterruptVariant::Complete(i) => {
                    let output = match i {
                        Ok(output) => output.into(),
                        Err(status_code) => Output {
                            status_code,
                            gas_left: 0,
                            output_data: Bytes::new(),
                            create_address: None,
                        },
                    };

                    if !T::DUMMY {
                        tracer.notify_execution_end(&output);
                    }

                    return output;
                }
            };
        }
    }

    pub fn execute_resumable(
        &self,
        trace: bool,
        message: Message,
        revision: Revision,
    ) -> ExecutionStartInterrupt {
        let code = self.clone();
        let inner = Box::pin(Gen::new(move |co| {
            interpreter_producer(co, code, ExecutionState::new(message, revision), trace)
        }));

        ExecutionStartInterrupt { inner, data: () }
    }
}

async fn interpreter_producer(
    mut co: Co<InterruptDataVariant, ResumeDataVariant>,
    s: AnalyzedCode,
    mut state: ExecutionState,
    trace: bool,
) -> Result<SuccessfulOutput, StatusCode> {
    let state = &mut state;

    let instruction_table = get_baseline_instruction_table(state.evm_revision);

    let mut reverted = false;

    let mut pc = 0;

    loop {
        let op = OpCode(s.code[pc]);

        // Do not print stop on the final STOP
        if trace && pc != s.code.len() - 1 {
            co.yield_(InterruptDataVariant::InstructionStart(Box::new(
                InstructionStart {
                    pc,
                    opcode: op,
                    state: state.clone(),
                },
            )))
            .await;
        }

        check_requirements(instruction_table, state, op)?;

        match op {
            OpCode::STOP => {
                break;
            }
            OpCode::ADD => {
                arithmetic::add(&mut state.stack);
            }
            OpCode::MUL => {
                arithmetic::mul(&mut state.stack);
            }
            OpCode::SUB => {
                arithmetic::sub(&mut state.stack);
            }
            OpCode::DIV => {
                arithmetic::div(&mut state.stack);
            }
            OpCode::SDIV => {
                arithmetic::sdiv(&mut state.stack);
            }
            OpCode::MOD => {
                arithmetic::modulo(&mut state.stack);
            }
            OpCode::SMOD => {
                arithmetic::smod(&mut state.stack);
            }
            OpCode::ADDMOD => {
                arithmetic::addmod(&mut state.stack);
            }
            OpCode::MULMOD => {
                arithmetic::mulmod(&mut state.stack);
            }
            OpCode::EXP => {
                arithmetic::exp(state)?;
            }
            OpCode::SIGNEXTEND => {
                arithmetic::signextend(&mut state.stack);
            }
            OpCode::LT => {
                boolean::lt(&mut state.stack);
            }
            OpCode::GT => {
                boolean::gt(&mut state.stack);
            }
            OpCode::SLT => {
                boolean::slt(&mut state.stack);
            }
            OpCode::SGT => {
                boolean::sgt(&mut state.stack);
            }
            OpCode::EQ => {
                boolean::eq(&mut state.stack);
            }
            OpCode::ISZERO => {
                boolean::iszero(&mut state.stack);
            }
            OpCode::AND => {
                boolean::and(&mut state.stack);
            }
            OpCode::OR => {
                boolean::or(&mut state.stack);
            }
            OpCode::XOR => {
                boolean::xor(&mut state.stack);
            }
            OpCode::NOT => {
                boolean::not(&mut state.stack);
            }
            OpCode::BYTE => {
                bitwise::byte(&mut state.stack);
            }
            OpCode::SHL => {
                bitwise::shl(&mut state.stack);
            }
            OpCode::SHR => {
                bitwise::shr(&mut state.stack);
            }
            OpCode::SAR => {
                bitwise::sar(&mut state.stack);
            }

            OpCode::KECCAK256 => {
                memory::keccak256(state)?;
            }
            OpCode::ADDRESS => {
                external::address(state);
            }
            OpCode::BALANCE => {
                balance!(co, state);
            }
            OpCode::CALLER => {
                external::caller(state);
            }
            OpCode::CALLVALUE => {
                external::callvalue(state);
            }
            OpCode::CALLDATALOAD => {
                calldataload(state);
            }
            OpCode::CALLDATASIZE => {
                calldatasize(state);
            }
            OpCode::CALLDATACOPY => {
                memory::calldatacopy(state)?;
            }
            OpCode::CODESIZE => {
                memory::codesize(&mut state.stack, &*s.code);
            }
            OpCode::CODECOPY => {
                memory::codecopy(state, &*s.code)?;
            }
            OpCode::EXTCODESIZE => {
                extcodesize!(co, state);
            }
            OpCode::EXTCODECOPY => {
                extcodecopy!(co, state);
            }
            OpCode::RETURNDATASIZE => {
                memory::returndatasize(state);
            }
            OpCode::RETURNDATACOPY => {
                memory::returndatacopy(state)?;
            }
            OpCode::EXTCODEHASH => {
                extcodehash!(co, state);
            }
            OpCode::BLOCKHASH => {
                blockhash!(co, state);
            }
            OpCode::ORIGIN
            | OpCode::COINBASE
            | OpCode::GASPRICE
            | OpCode::TIMESTAMP
            | OpCode::NUMBER
            | OpCode::DIFFICULTY
            | OpCode::GASLIMIT
            | OpCode::CHAINID
            | OpCode::BASEFEE => {
                push_txcontext!(
                    co,
                    state,
                    match op {
                        OpCode::ORIGIN => external::origin_accessor,
                        OpCode::COINBASE => external::coinbase_accessor,
                        OpCode::GASPRICE => external::gasprice_accessor,
                        OpCode::TIMESTAMP => external::timestamp_accessor,
                        OpCode::NUMBER => external::number_accessor,
                        OpCode::DIFFICULTY => external::difficulty_accessor,
                        OpCode::GASLIMIT => external::gaslimit_accessor,
                        OpCode::CHAINID => external::chainid_accessor,
                        OpCode::BASEFEE => external::basefee_accessor,
                        _ => unreachable!(),
                    }
                );
            }
            OpCode::SELFBALANCE => {
                selfbalance!(co, state);
            }
            OpCode::POP => {
                stack_manip::pop(&mut state.stack);
            }
            OpCode::MLOAD => {
                memory::mload(state)?;
            }
            OpCode::MSTORE => {
                memory::mstore(state)?;
            }
            OpCode::MSTORE8 => {
                memory::mstore8(state)?;
            }
            OpCode::JUMP => {
                pc = op_jump(state, &s.jumpdest_map)?;

                continue;
            }
            OpCode::JUMPI => {
                if !state.stack.get(1).is_zero() {
                    pc = op_jump(state, &s.jumpdest_map)?;
                    state.stack.pop();

                    continue;
                } else {
                    state.stack.pop();
                    state.stack.pop();
                }
            }
            OpCode::PC => state.stack.push(pc.into()),
            OpCode::MSIZE => memory::msize(state),
            OpCode::SLOAD => {
                sload!(co, state);
            }
            OpCode::SSTORE => {
                sstore!(co, state);
            }
            OpCode::GAS => state.stack.push(state.gas_left.into()),
            OpCode::JUMPDEST => {}
            OpCode::PUSH1
            | OpCode::PUSH2
            | OpCode::PUSH3
            | OpCode::PUSH4
            | OpCode::PUSH5
            | OpCode::PUSH6
            | OpCode::PUSH7
            | OpCode::PUSH8
            | OpCode::PUSH9
            | OpCode::PUSH10
            | OpCode::PUSH11
            | OpCode::PUSH12
            | OpCode::PUSH13
            | OpCode::PUSH14
            | OpCode::PUSH15
            | OpCode::PUSH16
            | OpCode::PUSH17
            | OpCode::PUSH18
            | OpCode::PUSH19
            | OpCode::PUSH20
            | OpCode::PUSH21
            | OpCode::PUSH22
            | OpCode::PUSH23
            | OpCode::PUSH24
            | OpCode::PUSH25
            | OpCode::PUSH26
            | OpCode::PUSH27
            | OpCode::PUSH28
            | OpCode::PUSH29
            | OpCode::PUSH30
            | OpCode::PUSH31
            | OpCode::PUSH32 => {
                pc += load_push(
                    &mut state.stack,
                    &s.code[pc + 1..],
                    op.to_usize() - OpCode::PUSH1.to_usize() + 1,
                )
            }

            OpCode::DUP1
            | OpCode::DUP2
            | OpCode::DUP3
            | OpCode::DUP4
            | OpCode::DUP5
            | OpCode::DUP6
            | OpCode::DUP7
            | OpCode::DUP8
            | OpCode::DUP9
            | OpCode::DUP10
            | OpCode::DUP11
            | OpCode::DUP12
            | OpCode::DUP13
            | OpCode::DUP14
            | OpCode::DUP15
            | OpCode::DUP16 => {
                dup(
                    &mut state.stack,
                    op.to_usize() - OpCode::DUP1.to_usize() + 1,
                );
            }

            OpCode::SWAP1
            | OpCode::SWAP2
            | OpCode::SWAP3
            | OpCode::SWAP4
            | OpCode::SWAP5
            | OpCode::SWAP6
            | OpCode::SWAP7
            | OpCode::SWAP8
            | OpCode::SWAP9
            | OpCode::SWAP10
            | OpCode::SWAP11
            | OpCode::SWAP12
            | OpCode::SWAP13
            | OpCode::SWAP14
            | OpCode::SWAP15
            | OpCode::SWAP16 => swap(
                &mut state.stack,
                op.to_usize() - OpCode::SWAP1.to_usize() + 1,
            ),

            OpCode::LOG0 | OpCode::LOG1 | OpCode::LOG2 | OpCode::LOG3 | OpCode::LOG4 => {
                do_log!(co, state, op.to_usize() - OpCode::LOG0.to_usize());
            }
            OpCode::CREATE | OpCode::CREATE2 => {
                do_create!(co, state, op == OpCode::CREATE2);
            }
            OpCode::CALL | OpCode::CALLCODE | OpCode::DELEGATECALL | OpCode::STATICCALL => {
                do_call!(
                    co,
                    state,
                    match op {
                        OpCode::CALL | OpCode::STATICCALL => CallKind::Call,
                        OpCode::CALLCODE => CallKind::CallCode,
                        OpCode::DELEGATECALL => CallKind::DelegateCall,
                        _ => unreachable!(),
                    },
                    op == OpCode::STATICCALL
                );
            }
            OpCode::RETURN | OpCode::REVERT => {
                ret(state)?;
                reverted = op == OpCode::REVERT;
                break;
            }
            OpCode::INVALID => {
                return Err(StatusCode::InvalidInstruction);
            }
            OpCode::SELFDESTRUCT => {
                selfdestruct!(co, state);
                break;
            }
            other => {
                unreachable!("reached unhandled opcode: {}", other);
            }
        }

        pc += 1;
    }

    let output = SuccessfulOutput {
        reverted,
        gas_left: state.gas_left,
        output_data: state.output_data.clone(),
    };

    Ok(output)
}
