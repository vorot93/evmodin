use self::instruction_table::*;
use crate::{
    instructions::{control::*, stack_manip::*, *},
    state::*,
    tracing::Tracer,
    *,
};
use ethereum_types::U256;

fn check_requirements(
    instruction_table: &InstructionTable,
    state: &mut ExecutionState,
    op: OpCode,
) -> StatusCode {
    let metrics = if let Some(v) = instruction_table[op.to_usize()] {
        v
    } else {
        return StatusCode::UndefinedInstruction;
    };

    state.gas_left -= metrics.gas_cost as i64;
    if state.gas_left < 0 {
        return StatusCode::OutOfGas;
    }

    let stack_size = state.stack.len();
    if stack_size == Stack::limit() {
        if metrics.can_overflow_stack {
            return StatusCode::StackOverflow;
        }
    } else if stack_size < metrics.stack_height_required.into() {
        return StatusCode::StackUnderflow;
    }

    StatusCode::Success
}

pub struct JumpdestMap(Vec<bool>);

impl JumpdestMap {
    pub fn contains(&self, dst: U256) -> bool {
        dst < self.0.len().into() && self.0[dst.as_usize()]
    }
}

/// Code with analysis.
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
    pub async fn execute<H: Host, T: Tracer>(
        &self,
        host: &mut H,
        tracer: &mut T,
        message: Message,
        revision: Revision,
    ) -> anyhow::Result<Output> {
        let state = &mut ExecutionState::new(message, revision);
        self.execute_with_state(host, tracer, state).await
    }

    pub(crate) async fn execute_with_state<H: Host, T: Tracer>(
        &self,
        host: &mut H,
        tracer: &mut T,
        state: &mut ExecutionState,
    ) -> anyhow::Result<Output> {
        tracer.notify_execution_start(state.evm_revision, state.message.clone(), self.code.clone());

        let instruction_table = get_baseline_instruction_table(state.evm_revision);

        let mut status;

        let mut pc = 0;

        loop {
            let op = OpCode(self.code[pc]);

            // Do not print stop on the final STOP
            if !T::DUMMY && pc != self.code.len() - 1 {
                tracer.notify_instruction_start(pc, op, state);
            }

            status = check_requirements(instruction_table, state, op);
            if !matches!(status, StatusCode::Success) {
                break;
            }

            let mut res = InstructionResolution::Continue;

            match op {
                OpCode::STOP => {
                    res = InstructionResolution::Exit(StatusCode::Success);
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
                    let status_code = arithmetic::exp(state);
                    if status_code != StatusCode::Success {
                        res = InstructionResolution::Exit(status_code);
                    }
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
                    res = memory::keccak256(state);
                }
                OpCode::ADDRESS => {
                    external::address(state);
                }
                OpCode::BALANCE => {
                    res = external::balance(host, state).await?;
                }
                OpCode::ORIGIN => {
                    external::origin(host, state).await?;
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
                    res = memory::calldatacopy(state);
                }
                OpCode::CODESIZE => {
                    memory::codesize(&mut state.stack, &*self.code);
                }
                OpCode::CODECOPY => {
                    res = memory::codecopy(state, &*self.code);
                }
                OpCode::GASPRICE => {
                    external::gasprice(host, state).await?;
                }
                OpCode::EXTCODESIZE => {
                    res = external::extcodesize(host, state).await?;
                }
                OpCode::EXTCODECOPY => {
                    res = memory::extcodecopy(host, state).await?;
                }
                OpCode::RETURNDATASIZE => {
                    memory::returndatasize(state);
                }
                OpCode::RETURNDATACOPY => {
                    res = memory::returndatacopy(state);
                }
                OpCode::EXTCODEHASH => {
                    res = memory::extcodehash(host, state).await?;
                }
                OpCode::BLOCKHASH => {
                    external::blockhash(host, state).await?;
                }
                OpCode::COINBASE => {
                    external::coinbase(host, state).await?;
                }
                OpCode::TIMESTAMP => {
                    external::timestamp(host, state).await?;
                }
                OpCode::NUMBER => {
                    external::number(host, state).await?;
                }
                OpCode::DIFFICULTY => {
                    external::difficulty(host, state).await?;
                }
                OpCode::GASLIMIT => {
                    external::gaslimit(host, state).await?;
                }
                OpCode::CHAINID => {
                    external::chainid(host, state).await?;
                }
                OpCode::SELFBALANCE => {
                    external::selfbalance(host, state).await?;
                }
                OpCode::BASEFEE => {
                    external::basefee(host, state).await?;
                }
                OpCode::POP => {
                    stack_manip::pop(&mut state.stack);
                }
                OpCode::MLOAD => {
                    let status_code = memory::mload(state);
                    if !matches!(status_code, StatusCode::Success) {
                        res = InstructionResolution::Exit(status_code);
                    }
                }
                OpCode::MSTORE => {
                    let status_code = memory::mstore(state);
                    if !matches!(status_code, StatusCode::Success) {
                        res = InstructionResolution::Exit(status_code);
                    }
                }
                OpCode::MSTORE8 => {
                    let status_code = memory::mstore8(state);
                    if !matches!(status_code, StatusCode::Success) {
                        res = InstructionResolution::Exit(status_code);
                    }
                }

                OpCode::JUMP => {
                    res = op_jump(state, &self.jumpdest_map);
                }
                OpCode::JUMPI => {
                    let v = state.stack.get(1);
                    if !v.is_zero() {
                        res = op_jump(state, &self.jumpdest_map);
                    } else {
                        state.stack.pop();
                    }
                    state.stack.pop();
                }
                OpCode::PC => state.stack.push(pc.into()),
                OpCode::MSIZE => memory::msize(state),
                OpCode::SLOAD => {
                    res = external::sload(host, state).await?;
                }
                OpCode::SSTORE => {
                    res = external::sstore(host, state).await?;
                }
                OpCode::GAS => state.stack.push(state.gas_left.into()),
                OpCode::JUMPDEST => {}
                OpCode::PUSH1 => pc += load_push::<1>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH2 => pc += load_push::<2>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH3 => pc += load_push::<3>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH4 => pc += load_push::<4>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH5 => pc += load_push::<5>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH6 => pc += load_push::<6>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH7 => pc += load_push::<7>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH8 => pc += load_push::<8>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH9 => pc += load_push::<9>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH10 => pc += load_push::<10>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH11 => pc += load_push::<11>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH12 => pc += load_push::<12>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH13 => pc += load_push::<13>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH14 => pc += load_push::<14>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH15 => pc += load_push::<15>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH16 => pc += load_push::<16>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH17 => pc += load_push::<17>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH18 => pc += load_push::<18>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH19 => pc += load_push::<19>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH20 => pc += load_push::<20>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH21 => pc += load_push::<21>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH22 => pc += load_push::<22>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH23 => pc += load_push::<23>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH24 => pc += load_push::<24>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH25 => pc += load_push::<25>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH26 => pc += load_push::<26>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH27 => pc += load_push::<27>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH28 => pc += load_push::<28>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH29 => pc += load_push::<29>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH30 => pc += load_push::<30>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH31 => pc += load_push::<31>(&mut state.stack, &self.code[pc + 1..]),
                OpCode::PUSH32 => pc += load_push::<32>(&mut state.stack, &self.code[pc + 1..]),

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

                OpCode::LOG0 => res = external::log::<_, 0>(host, state).await?,
                OpCode::LOG1 => res = external::log::<_, 1>(host, state).await?,
                OpCode::LOG2 => res = external::log::<_, 2>(host, state).await?,
                OpCode::LOG3 => res = external::log::<_, 3>(host, state).await?,
                OpCode::LOG4 => res = external::log::<_, 4>(host, state).await?,

                OpCode::CREATE => res = call::create(host, state, false).await?,
                OpCode::CALL => res = call::call(host, state, CallKind::Call, false).await?,
                OpCode::CALLCODE => {
                    res = call::call(host, state, CallKind::CallCode, false).await?
                }
                OpCode::RETURN => res = ret(state, StatusCode::Success),
                OpCode::DELEGATECALL => {
                    res = call::call(host, state, CallKind::DelegateCall, false).await?
                }
                OpCode::STATICCALL => res = call::call(host, state, CallKind::Call, true).await?,
                OpCode::CREATE2 => res = call::create(host, state, true).await?,
                OpCode::REVERT => res = ret(state, StatusCode::Revert),
                OpCode::INVALID => {
                    res = InstructionResolution::Exit(StatusCode::InvalidInstruction)
                }
                OpCode::SELFDESTRUCT => {
                    res = external::selfdestruct(host, state).await?;
                }
                other => {
                    unreachable!("reached unhandled opcode: {}", other);
                }
            }

            match res {
                InstructionResolution::Continue => pc += 1,
                InstructionResolution::Exit(code) => {
                    status = code;
                    break;
                }
                InstructionResolution::Jump(offset) => pc = offset,
            }
        }

        let gas_left = match status {
            StatusCode::Success | StatusCode::Revert => state.gas_left,
            _ => 0,
        };

        let output = Output {
            status_code: status,
            gas_left,
            output_data: state.output_data.clone(),
            create_address: None,
        };

        tracer.notify_execution_end(&output);

        Ok(output)
    }
}
