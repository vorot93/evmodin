use self::instruction_table::*;
use crate::{
    common::*,
    continuation::{interrupt::*, interrupt_data::*, resume_data::*, *},
    instructions::{control::*, stack_manip::*, *},
    state::*,
    tracing::Tracer,
    *,
};
use ethnum::U256;
use std::{ops::Generator, sync::Arc};

fn check_requirements(
    instruction_table: &InstructionTable,
    state: &mut ExecutionState,
    op: OpCode,
) -> Result<(), StatusCode> {
    let metrics = instruction_table[op.to_usize()]
        .as_ref()
        .ok_or(StatusCode::UndefinedInstruction)?;

    state.gas_left -= metrics.gas_cost as i64;
    if state.gas_left < 0 {
        return Err(StatusCode::OutOfGas);
    }

    let stack_size = state.stack.len();
    if stack_size == STACK_SIZE {
        if metrics.can_overflow_stack {
            return Err(StatusCode::StackOverflow);
        }
    } else if stack_size < metrics.stack_height_required.into() {
        return Err(StatusCode::StackUnderflow);
    }

    Ok(())
}

#[derive(Clone, Debug)]
pub struct JumpdestMap(Arc<[bool]>);

impl JumpdestMap {
    pub fn contains(&self, dst: U256) -> bool {
        dst < u128::try_from(self.0.len()).unwrap() && self.0[dst.as_usize()]
    }
}

/// Code with analysis.
#[derive(Clone, Debug)]
pub struct AnalyzedCode {
    jumpdest_map: JumpdestMap,
    code: Bytes,
    padded_code: Bytes,
}

impl AnalyzedCode {
    /// Analyze code and prepare it for execution.
    pub fn analyze(code: impl Into<Vec<u8>>) -> Self {
        let code = code.into();
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

        let code_len = code.len();

        let mut padded_code = code;
        padded_code.resize(i + 1, OpCode::STOP.to_u8());

        let jumpdest_map = JumpdestMap(jumpdest_map.into());
        let padded_code = Bytes::from(padded_code);
        let mut code = padded_code.clone();
        code.truncate(code_len);

        Self {
            jumpdest_map,
            code,
            padded_code,
        }
    }

    /// Execute analyzed EVM bytecode using provided `Host` context. Optionally modify the state after each instruction using provided closure.
    pub fn execute<H: Host, T: Tracer>(
        &self,
        host: &mut H,
        tracer: &mut T,
        state_modifier: StateModifier,
        message: Message,
        revision: Revision,
    ) -> Output {
        if !T::DUMMY {
            tracer.notify_execution_start(revision, message.clone(), self.code.clone());
        }

        let output = self
            .execute_resumable(!T::DUMMY || state_modifier.is_some(), message, revision)
            .run_to_completion_with_host(host, tracer, state_modifier);

        if !T::DUMMY {
            tracer.notify_execution_end(&output);
        }

        output
    }

    /// Execute in resumable EVM.
    pub fn execute_resumable(
        &self,
        trace: bool,
        message: Message,
        revision: Revision,
    ) -> ExecutionStartInterrupt {
        let code = self.clone();
        let state = ExecutionState::new(message);
        let f = match (trace, revision) {
            (true, Revision::Frontier) => interpreter_producer::<true, { Revision::Frontier }>,
            (true, Revision::Homestead) => interpreter_producer::<true, { Revision::Homestead }>,
            (true, Revision::Tangerine) => interpreter_producer::<true, { Revision::Tangerine }>,
            (true, Revision::Spurious) => interpreter_producer::<true, { Revision::Spurious }>,
            (true, Revision::Byzantium) => interpreter_producer::<true, { Revision::Byzantium }>,
            (true, Revision::Constantinople) => {
                interpreter_producer::<true, { Revision::Constantinople }>
            }
            (true, Revision::Petersburg) => interpreter_producer::<true, { Revision::Petersburg }>,
            (true, Revision::Istanbul) => interpreter_producer::<true, { Revision::Istanbul }>,
            (true, Revision::Berlin) => interpreter_producer::<true, { Revision::Berlin }>,
            (true, Revision::London) => interpreter_producer::<true, { Revision::London }>,
            (true, Revision::Shanghai) => interpreter_producer::<true, { Revision::Shanghai }>,
            (false, Revision::Frontier) => interpreter_producer::<false, { Revision::Frontier }>,
            (false, Revision::Homestead) => interpreter_producer::<false, { Revision::Homestead }>,
            (false, Revision::Tangerine) => interpreter_producer::<false, { Revision::Tangerine }>,
            (false, Revision::Spurious) => interpreter_producer::<false, { Revision::Spurious }>,
            (false, Revision::Byzantium) => interpreter_producer::<false, { Revision::Byzantium }>,
            (false, Revision::Constantinople) => {
                interpreter_producer::<false, { Revision::Constantinople }>
            }
            (false, Revision::Petersburg) => {
                interpreter_producer::<false, { Revision::Petersburg }>
            }
            (false, Revision::Istanbul) => interpreter_producer::<false, { Revision::Istanbul }>,
            (false, Revision::Berlin) => interpreter_producer::<false, { Revision::Berlin }>,
            (false, Revision::London) => interpreter_producer::<false, { Revision::London }>,
            (false, Revision::Shanghai) => interpreter_producer::<false, { Revision::Shanghai }>,
        };
        let inner = (f)(code, state);

        ExecutionStartInterrupt { inner }
    }
}

impl ExecutionStartInterrupt {
    #[doc(hidden)]
    pub fn run_to_completion_with_host2<H: Host, T: Tracer>(
        self,
        host: &mut H,
        tracer: &mut T,
        state_modifier: StateModifier,
    ) -> (Output, ExecutionComplete) {
        let mut interrupt = self.resume(());

        loop {
            interrupt = match interrupt {
                InterruptVariant::InstructionStart(data, i) => {
                    tracer.notify_instruction_start(data.pc, data.opcode, &data.state);
                    i.resume(state_modifier.clone())
                }
                InterruptVariant::AccountExists(data, i) => {
                    let exists = host.account_exists(data.address);
                    i.resume(AccountExistsStatus { exists })
                }
                InterruptVariant::GetBalance(data, i) => {
                    let balance = host.get_balance(data.address);
                    i.resume(Balance { balance })
                }
                InterruptVariant::GetCodeSize(data, i) => {
                    let code_size = host.get_code_size(data.address);
                    i.resume(CodeSize { code_size })
                }
                InterruptVariant::GetStorage(data, i) => {
                    let value = host.get_storage(data.address, data.key);
                    i.resume(StorageValue { value })
                }
                InterruptVariant::SetStorage(data, i) => {
                    let status = host.set_storage(data.address, data.key, data.value);
                    i.resume(StorageStatusInfo { status })
                }
                InterruptVariant::GetCodeHash(data, i) => {
                    let hash = host.get_code_hash(data.address);
                    i.resume(CodeHash { hash })
                }
                InterruptVariant::CopyCode(data, i) => {
                    let mut code = vec![0; data.max_size];
                    let copied = host.copy_code(data.address, data.offset, &mut code[..]);
                    debug_assert!(copied <= code.len());
                    code.truncate(copied);
                    let code = code.into();
                    i.resume(Code { code })
                }
                InterruptVariant::Selfdestruct(data, i) => {
                    host.selfdestruct(data.address, data.beneficiary);
                    i.resume(())
                }
                InterruptVariant::Call(data, i) => {
                    let message = match data {
                        Call::Call(message) => message,
                        Call::Create(message) => message.into(),
                    };
                    let output = host.call(&message);
                    i.resume(CallOutput { output })
                }
                InterruptVariant::GetTxContext(i) => {
                    let context = host.get_tx_context();
                    i.resume(TxContextData { context })
                }
                InterruptVariant::GetBlockHash(data, i) => {
                    let hash = host.get_block_hash(data.block_number);
                    i.resume(BlockHash { hash })
                }
                InterruptVariant::EmitLog(data, i) => {
                    host.emit_log(data.address, &*data.data, data.topics.as_slice());
                    i.resume(())
                }
                InterruptVariant::AccessAccount(data, i) => {
                    let status = host.access_account(data.address);
                    i.resume(AccessAccountStatus { status })
                }
                InterruptVariant::AccessStorage(data, i) => {
                    let status = host.access_storage(data.address, data.key);
                    i.resume(AccessStorageStatus { status })
                }
                InterruptVariant::Complete(i, c) => {
                    let output = match i {
                        Ok(output) => output.into(),
                        Err(status_code) => Output {
                            status_code,
                            gas_left: 0,
                            output_data: Bytes::new(),
                            create_address: None,
                        },
                    };

                    return (output, c);
                }
            };
        }
    }

    pub fn run_to_completion_with_host<H: Host, T: Tracer>(
        self,
        host: &mut H,
        tracer: &mut T,
        state_modifier: StateModifier,
    ) -> Output {
        self.run_to_completion_with_host2(host, tracer, state_modifier)
            .0
    }
}

fn interpreter_producer<const TRACE: bool, const REVISION: Revision>(
    s: AnalyzedCode,
    mut state: ExecutionState,
) -> Box<
    dyn Generator<
            ResumeDataVariant,
            Yield = InterruptDataVariant,
            Return = Result<SuccessfulOutput, StatusCode>,
        > + Send
        + Sync
        + Unpin,
> {
    Box::new(move |_: ResumeDataVariant| {
        let instruction_table = get_baseline_instruction_table(REVISION);

        let mut reverted = false;

        let mut pc = 0;

        loop {
            let op = OpCode(s.padded_code[pc]);

            if TRACE {
                // Do not print stop on the final STOP
                if pc < s.code.len() {
                    if let Some(modifier) = {
                        yield InterruptDataVariant::InstructionStart(Box::new(InstructionStart {
                            pc,
                            opcode: op,
                            state: state.clone(),
                        }))
                    }
                    .as_state_modifier()
                    .unwrap()
                    {
                        (modifier)(&mut state)
                    }
                }
            }

            check_requirements(instruction_table, &mut state, op)?;

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
                    arithmetic::exp::<REVISION>(&mut state)?;
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
                    memory::keccak256(&mut state)?;
                }
                OpCode::ADDRESS => {
                    external::address(&mut state);
                }
                OpCode::BALANCE => {
                    balance!(&mut state, REVISION);
                }
                OpCode::CALLER => {
                    external::caller(&mut state);
                }
                OpCode::CALLVALUE => {
                    external::callvalue(&mut state);
                }
                OpCode::CALLDATALOAD => {
                    calldataload(&mut state);
                }
                OpCode::CALLDATASIZE => {
                    calldatasize(&mut state);
                }
                OpCode::CALLDATACOPY => {
                    memory::calldatacopy(&mut state)?;
                }
                OpCode::CODESIZE => {
                    memory::codesize(&mut state.stack, &s.code[..]);
                }
                OpCode::CODECOPY => {
                    memory::codecopy(&mut state, &s.code[..])?;
                }
                OpCode::EXTCODESIZE => {
                    extcodesize!(&mut state, REVISION);
                }
                OpCode::EXTCODECOPY => {
                    extcodecopy!(state, REVISION);
                }
                OpCode::RETURNDATASIZE => {
                    memory::returndatasize(&mut state);
                }
                OpCode::RETURNDATACOPY => {
                    memory::returndatacopy(&mut state)?;
                }
                OpCode::EXTCODEHASH => {
                    extcodehash!(state, REVISION);
                }
                OpCode::BLOCKHASH => {
                    blockhash!(state);
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
                    selfbalance!(state);
                }
                OpCode::POP => pop(&mut state.stack),
                OpCode::MLOAD => memory::mload(&mut state)?,
                OpCode::MSTORE => memory::mstore(&mut state)?,
                OpCode::MSTORE8 => memory::mstore8(&mut state)?,
                OpCode::JUMP => {
                    pc = op_jump(&mut state, &s.jumpdest_map)?;

                    continue;
                }
                OpCode::JUMPI => {
                    if *state.stack.get(1) != 0 {
                        pc = op_jump(&mut state, &s.jumpdest_map)?;
                        state.stack.pop();

                        continue;
                    } else {
                        state.stack.pop();
                        state.stack.pop();
                    }
                }
                OpCode::PC => state.stack.push(u128::try_from(pc).unwrap().into()),
                OpCode::MSIZE => memory::msize(&mut state),
                OpCode::SLOAD => {
                    sload!(state, REVISION);
                }
                OpCode::SSTORE => {
                    sstore!(state, REVISION);
                }
                OpCode::GAS => state
                    .stack
                    .push(u128::try_from(state.gas_left).unwrap().into()),
                OpCode::JUMPDEST => {}
                OpCode::PUSH1 => {
                    push1(&mut state.stack, s.padded_code[pc + 1]);
                    pc += 1;
                }
                OpCode::PUSH2 => pc += push::<2>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH3 => pc += push::<3>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH4 => pc += push::<4>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH5 => pc += push::<5>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH6 => pc += push::<6>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH7 => pc += push::<7>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH8 => pc += push::<8>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH9 => pc += push::<9>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH10 => pc += push::<10>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH11 => pc += push::<11>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH12 => pc += push::<12>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH13 => pc += push::<13>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH14 => pc += push::<14>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH15 => pc += push::<15>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH16 => pc += push::<16>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH17 => pc += push::<17>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH18 => pc += push::<18>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH19 => pc += push::<19>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH20 => pc += push::<20>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH21 => pc += push::<21>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH22 => pc += push::<22>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH23 => pc += push::<23>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH24 => pc += push::<24>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH25 => pc += push::<25>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH26 => pc += push::<26>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH27 => pc += push::<27>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH28 => pc += push::<28>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH29 => pc += push::<29>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH30 => pc += push::<30>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH31 => pc += push::<31>(&mut state.stack, &s.padded_code[pc + 1..]),
                OpCode::PUSH32 => {
                    push32(&mut state.stack, &s.padded_code[pc + 1..]);
                    pc += 32;
                }

                OpCode::DUP1 => dup::<1>(&mut state.stack),
                OpCode::DUP2 => dup::<2>(&mut state.stack),
                OpCode::DUP3 => dup::<3>(&mut state.stack),
                OpCode::DUP4 => dup::<4>(&mut state.stack),
                OpCode::DUP5 => dup::<5>(&mut state.stack),
                OpCode::DUP6 => dup::<6>(&mut state.stack),
                OpCode::DUP7 => dup::<7>(&mut state.stack),
                OpCode::DUP8 => dup::<8>(&mut state.stack),
                OpCode::DUP9 => dup::<9>(&mut state.stack),
                OpCode::DUP10 => dup::<10>(&mut state.stack),
                OpCode::DUP11 => dup::<11>(&mut state.stack),
                OpCode::DUP12 => dup::<12>(&mut state.stack),
                OpCode::DUP13 => dup::<13>(&mut state.stack),
                OpCode::DUP14 => dup::<14>(&mut state.stack),
                OpCode::DUP15 => dup::<15>(&mut state.stack),
                OpCode::DUP16 => dup::<16>(&mut state.stack),

                OpCode::SWAP1 => swap::<1>(&mut state.stack),
                OpCode::SWAP2 => swap::<2>(&mut state.stack),
                OpCode::SWAP3 => swap::<3>(&mut state.stack),
                OpCode::SWAP4 => swap::<4>(&mut state.stack),
                OpCode::SWAP5 => swap::<5>(&mut state.stack),
                OpCode::SWAP6 => swap::<6>(&mut state.stack),
                OpCode::SWAP7 => swap::<7>(&mut state.stack),
                OpCode::SWAP8 => swap::<8>(&mut state.stack),
                OpCode::SWAP9 => swap::<9>(&mut state.stack),
                OpCode::SWAP10 => swap::<10>(&mut state.stack),
                OpCode::SWAP11 => swap::<11>(&mut state.stack),
                OpCode::SWAP12 => swap::<12>(&mut state.stack),
                OpCode::SWAP13 => swap::<13>(&mut state.stack),
                OpCode::SWAP14 => swap::<14>(&mut state.stack),
                OpCode::SWAP15 => swap::<15>(&mut state.stack),
                OpCode::SWAP16 => swap::<16>(&mut state.stack),

                OpCode::LOG0 | OpCode::LOG1 | OpCode::LOG2 | OpCode::LOG3 | OpCode::LOG4 => {
                    do_log!(&mut state, op.0 - OpCode::LOG0.0);
                }
                OpCode::CREATE | OpCode::CREATE2 => {
                    do_create!(&mut state, REVISION, op == OpCode::CREATE2);
                }
                OpCode::CALL | OpCode::CALLCODE | OpCode::DELEGATECALL | OpCode::STATICCALL => {
                    do_call!(
                        &mut state,
                        REVISION,
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
                    ret(&mut state)?;
                    reverted = op == OpCode::REVERT;
                    break;
                }
                OpCode::INVALID => {
                    return Err(StatusCode::InvalidInstruction);
                }
                OpCode::SELFDESTRUCT => {
                    selfdestruct!(state, REVISION);
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
    })
}
