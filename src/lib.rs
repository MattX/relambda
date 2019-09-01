// Copyright 2019 Matthieu Felix
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::borrow::Borrow;
use std::io::{stdin, Read};
use std::ops::Deref;
use std::rc::Rc;

use log::debug;
use unicode_reader::CodePoints;

use crate::parse::{parse_toplevel, Application, CharPosIterator, Combinator, SyntaxTree};

mod parse;

/// All values in Unlambda are formally unary functions.
///
/// In reality, some of these functions are semantically binary or ternary, but they're curried to
/// maintain unarity. Other functions are actually continuations or promises, but in any case,
/// they accept a single operation, unary application.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Function {
    /// Identity function. Returns the argument when applied.
    I,
    /// Kestrel or constant combinator. Returns a K1 that holds the argument when applied.
    K,
    /// Constant function. This holds the value that was passed to a K, and, when applied,
    /// discards the application argument and returns the stored value.
    K1(Rc<Function>),
    /// Starling. This is a three-argument function; _'''Sxyz_ evaluates to _''xz'yz_. Note that
    /// while this is straightforward in regular KSI calculus, if _'xz_ evaluates to `D`, the
    /// semantics of the outer application change, so extra care must be taken.
    S,
    /// First partial application of S.
    S1(Rc<Function>),
    /// Second partial application of S. When applied, performs the transformation described above.
    S2(Rc<Function>, Rc<Function>),
    /// Void. When applied, discards its argument and returns itself.
    V,
    /// Promise constructor. This is a special form rather than a function; when applied, its
    /// argument is not evaluated and is instead packaged into a promise. This mechanism is useful
    /// for delaying side-effects.
    D,
    /// Promise. When applied, `Expression` is evaluated and the result is substituted before the
    /// application proceeds.
    D1(Expression),
    /// call-with-current-continuation. This has the same semantics as in Scheme.
    C,
    /// A continuation.
    C1(Box<VmState>),
    /// Special continuation representing the whole program. When invoked, exits with the argument
    /// as a value.
    E,
    Read,
    Reprint,
    Compare(char),
    /// When invoked, acts like `I` but prints the attached char.
    Dot(char),
}

impl Function {
    fn from_combinator(c: Combinator) -> Self {
        match c {
            Combinator::I => Function::I,
            Combinator::K => Function::K,
            Combinator::S => Function::S,
            Combinator::V => Function::V,
            Combinator::D => Function::D,
            Combinator::C => Function::C,
            Combinator::E => Function::E,
            Combinator::Read => Function::Read,
            Combinator::Reprint => Function::Reprint,
            Combinator::Compare(ch) => Function::Compare(ch),
            Combinator::Dot(ch) => Function::Dot(ch),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Expression {
    Promise(usize),
    Function(Rc<Function>),
    Application(Rc<Function>, Rc<Function>),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum OpCode {
    /// Used during compilation phase to reserve a spot for an instruction that we don't know yet.
    Placeholder,
    /// Push the given combinator to the stack.
    PushImmediate(Combinator),
    /// Swap the two top values on the stack.
    Swap,
    /// Move the top stack value to third position, moving second and third to first and second,
    /// respectively.
    Rot,
    /// If the function about to be invoked is a d, create a promise instead of applying.
    CheckSuspend(usize),
    /// If the function about to be invoked is a d, abort invocation and use the promise on the
    /// stack.
    CheckDynamicSuspend(usize),
    /// Apply the value in top position of the stack to the value in second position.
    Invoke,
    /// Exit the program.
    Finish,
}

const S2_START: usize = 0;
const S2_LEN: usize = 5;
const S2_END: usize = S2_START + S2_LEN;
const S2_CODE: [OpCode; S2_LEN] = [
    OpCode::Invoke,
    OpCode::CheckDynamicSuspend(4),
    OpCode::Rot,
    OpCode::Invoke,
    OpCode::Invoke,
];

const D1_PROMISE_START: usize = S2_END;
const D1_PROMISE_LEN: usize = 2;
const D1_PROMISE_END: usize = D1_PROMISE_START + D1_PROMISE_LEN;
const D1_PROMISE_CODE: [OpCode; D1_PROMISE_LEN] = [OpCode::Swap, OpCode::Invoke];

const D1_APPLICATION_START: usize = D1_PROMISE_END;
const D1_APPLICATION_LEN: usize = 3;
const D1_APPLICATION_END: usize = D1_APPLICATION_START + D1_APPLICATION_LEN;
const D1_APPLICATION_CODE: [OpCode; D1_APPLICATION_LEN] =
    [OpCode::Invoke, OpCode::Swap, OpCode::Invoke];

/// Structure representing the state of the VM.
///
/// The return stack is a list of `(to, from)` tuples. If at any point during execution, the
/// program counter equals the `from` element of the topmost stack item, the element is dropped
/// and the program counter is set to `to`.
///
/// Always use `push_rstack` to add elements to the return stack, as it performs TCO. The TCO
/// invariant is that `stack[-1].to != stack[-2].from`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VmState {
    stack: Vec<Rc<Function>>,
    rstack: Vec<(usize, usize)>,
    pc: usize,
    cur_char: Option<char>,
}

impl VmState {
    fn push_rstack(&mut self, to: usize, from: usize) {
        let (then_to, then_from) = self.rstack[self.rstack.len() - 1];
        if then_from == to {
            let last = self.rstack.len() - 1;
            self.rstack[last] = (then_to, from);
        } else {
            self.rstack.push((to, from));
        }
        debug_assert_ne!(
            self.rstack[self.rstack.len() - 2].1,
            self.rstack[self.rstack.len() - 1].0
        );
    }
}

impl Default for VmState {
    fn default() -> Self {
        Self {
            stack: Vec::new(),
            rstack: Vec::new(),
            pc: 0,
            cur_char: None,
        }
    }
}

fn run_vm(code: &[OpCode], entry_point: usize) -> Result<Rc<Function>, String> {
    let mut vm_state = VmState::default();
    vm_state.pc = entry_point;

    // The loop expects a top element on the return stack in order to check for auto-returns.
    // Add a sentinel here that will never trigger, and would jump to an illegal location if it did.
    vm_state.rstack.push((code.len(), code.len()));

    loop {
        let opcode = code[vm_state.pc];
        match opcode {
            OpCode::Placeholder => panic!("placeholder not replaced during compilation"),
            OpCode::PushImmediate(c) => vm_state.stack.push(Rc::new(Function::from_combinator(c))),
            OpCode::Rot => {
                let (fst, snd, thr) = (
                    vm_state.stack.pop().unwrap(),
                    vm_state.stack.pop().unwrap(),
                    vm_state.stack.pop().unwrap(),
                );
                vm_state.stack.push(fst);
                vm_state.stack.push(thr);
                vm_state.stack.push(snd);
            }
            OpCode::Swap => {
                let (fst, snd) = (vm_state.stack.pop().unwrap(), vm_state.stack.pop().unwrap());
                vm_state.stack.push(fst);
                vm_state.stack.push(snd);
            }
            OpCode::CheckSuspend(offset) => {
                if vm_state.stack[vm_state.stack.len() - 1].deref() == &Function::D {
                    vm_state.stack.pop().unwrap();
                    vm_state
                        .stack
                        .push(Rc::new(Function::D1(Expression::Promise(vm_state.pc + 1))));
                    vm_state.pc += offset;
                } else {
                    vm_state.pc += 1;
                }
            }
            OpCode::CheckDynamicSuspend(offset) => {
                // During a CheckDynamicSuspend, the stack is guaranteed to be set up as
                // top→ (operator) (promise of operand) (operand's operator) (operand's operand)
                // If the operator is D, drop the operand members; otherwise, drop the promise.
                let operator = vm_state.stack.pop().unwrap();
                if operator.deref() == &Function::D {
                    let promise = vm_state.stack.pop().unwrap();
                    vm_state.stack.pop().unwrap();
                    vm_state.stack.pop().unwrap();
                    vm_state.stack.push(promise);
                    vm_state.pc += offset;
                } else {
                    vm_state.stack.pop().unwrap();
                    vm_state.stack.push(operator);
                    vm_state.pc += 1;
                }
            }
            OpCode::Invoke => {
                if let Some(ret) = invoke(code, &mut vm_state)? {
                    return Ok(ret);
                }
            }
            OpCode::Finish => {
                // The rstack should contain only our sentinel return point
                debug_assert_eq!(vm_state.stack.len(), 1);
                debug_assert_eq!(vm_state.rstack, [(code.len(), code.len())]);
                return Ok(vm_state.stack.pop().unwrap());
            }
        }
        match opcode {
            OpCode::Invoke | OpCode::CheckSuspend(_) | OpCode::CheckDynamicSuspend(_) => (),
            _ => vm_state.pc += 1,
        }
        debug!("{:?} ({:?} → {:?})", &vm_state, opcode, code[vm_state.pc]);

        let (to, from) = vm_state.rstack[vm_state.rstack.len() - 1];
        if vm_state.pc == from {
            debug!("Returning {} → {}", vm_state.pc, to);
            vm_state.pc = to;
            vm_state.rstack.pop();
        }
    }
}

fn invoke(code: &[OpCode], vm_state: &mut VmState) -> Result<Option<Rc<Function>>, String> {
    let (arg, fun) = (vm_state.stack.pop().unwrap(), vm_state.stack.pop().unwrap());
    match fun.borrow() {
        Function::I => vm_state.stack.push(arg),
        Function::K => vm_state.stack.push(Rc::new(Function::K1(arg))),
        Function::K1(val) => vm_state.stack.push(val.clone()),
        Function::S => vm_state.stack.push(Rc::new(Function::S1(arg))),
        Function::S1(val) => vm_state.stack.push(Rc::new(Function::S2(val.clone(), arg))),
        Function::S2(val1, val2) => {
            // We want to compute ``(val1)(arg)`(val2)(arg), evaluating `(val1)(arg) first.
            // If `(val1)(arg) evaluates to D, we'll need to create a promise instead of doing
            // a normal application. Push that promise on the stack; the S2 microcode will take
            // care of either dropping or using it.
            vm_state.stack.push(val2.clone());
            vm_state.stack.push(arg.clone());
            vm_state
                .stack
                .push(Rc::new(Function::D1(Expression::Application(
                    val2.clone(),
                    arg.clone(),
                ))));
            vm_state.stack.push(val1.clone());
            vm_state.stack.push(arg.clone());
            vm_state.push_rstack(vm_state.pc + 1, S2_END);
            vm_state.pc = S2_START;
        }
        Function::V => vm_state.stack.push(fun.clone()),
        Function::D => vm_state
            .stack
            .push(Rc::new(Function::D1(Expression::Function(arg)))),
        Function::D1(Expression::Promise(at)) => {
            // The promise object points to a location in the code which contains the necessary
            // instructions to force the promise. The instructions in question end just before
            // the CheckSuspend jump target. Once we are done forcing the promise, we need to
            // return into D1 microcode to perform the actual application.
            if let OpCode::CheckSuspend(offset) = code[*at - 1] {
                vm_state.stack.push(arg);
                vm_state.push_rstack(vm_state.pc + 1, D1_PROMISE_END);
                vm_state.push_rstack(D1_PROMISE_START, *at - 2 + offset);
                vm_state.pc = *at;
            } else {
                panic!("promise does not point to a CheckSuspend opcode");
            }
        }
        Function::D1(Expression::Function(f)) => {
            vm_state.stack.push(f.clone());
            vm_state.stack.push(arg);
        }
        Function::D1(Expression::Application(operator, operand)) => {
            vm_state.stack.push(arg);
            vm_state.stack.push(operator.clone());
            vm_state.stack.push(operand.clone());
            vm_state.push_rstack(vm_state.pc + 1, D1_APPLICATION_END);
            vm_state.pc = D1_APPLICATION_START;
        }
        Function::C => {
            let saved_state = vm_state.clone();
            vm_state.stack.push(arg);
            vm_state
                .stack
                .push(Rc::new(Function::C1(Box::new(saved_state))));
        }
        Function::C1(cont) => {
            vm_state.stack = cont.stack.clone();
            vm_state.stack.push(arg);
            vm_state.rstack = cont.rstack.clone();
            vm_state.pc = cont.pc;
        }
        Function::E => return Ok(Some(arg)),
        Function::Read => {
            let ch = CodePoints::from(stdin().bytes())
                .next()
                .and_then(|v| v.ok());
            vm_state.cur_char = ch;
            vm_state.stack.push(arg);
            vm_state.stack.push(Rc::new(if ch.is_some() {
                Function::I
            } else {
                Function::V
            }));
        }
        Function::Reprint => {
            let fun = vm_state.cur_char.map_or(Function::V, Function::Dot);
            vm_state.stack.push(arg);
            vm_state.stack.push(Rc::new(fun));
        }
        Function::Compare(ch) => {
            let is_same = vm_state.cur_char.map_or(false, |c| c == *ch);
            vm_state.stack.push(arg);
            vm_state
                .stack
                .push(Rc::new(if is_same { Function::I } else { Function::V }));
        }
        Function::Dot(ch) => {
            print!("{}", ch);
            vm_state.stack.push(arg);
        }
    }

    match fun.borrow() {
        // The following do not advance the pc because they've just set it
        Function::S2(_, _)
        | Function::D1(Expression::Promise(_))
        | Function::D1(Expression::Application(_, _)) => (),
        // The following do not advance the pc in order to call OpCode::Invoke again
        Function::C
        | Function::Read
        | Function::Reprint
        | Function::D1(Expression::Function(_)) => {
            debug_assert_eq!(code[vm_state.pc], OpCode::Invoke);
        }
        _ => vm_state.pc += 1,
    }

    Ok(None)
}

fn compile(st: &SyntaxTree, code: &mut Vec<OpCode>) -> Result<(), String> {
    match st {
        SyntaxTree::Combinator(c) => code.push(OpCode::PushImmediate(*c)),
        SyntaxTree::Application(Application { func, arg }) => {
            compile(func, code)?;
            let placeholder_position = code.len();
            code.push(OpCode::Placeholder);
            compile(arg, code)?;
            code.push(OpCode::Invoke);
            let next_position = code.len();
            code[placeholder_position] = OpCode::CheckSuspend(next_position - placeholder_position);
        }
    }
    Ok(())
}

fn compile_toplevel(st: &SyntaxTree) -> Result<(Vec<OpCode>, usize), String> {
    let mut code = S2_CODE.to_vec();
    code.extend_from_slice(&D1_PROMISE_CODE);
    code.extend_from_slice(&D1_APPLICATION_CODE);
    let entry_point = code.len();
    compile(st, &mut code)?;
    code.push(OpCode::Finish);
    debug!(
        "Compiled: {:?}",
        code.iter().enumerate().collect::<Vec<_>>()
    );
    Ok((code, entry_point))
}

pub fn parse_compile_run(code: &str) -> Result<Function, String> {
    let st = parse_toplevel(&mut CharPosIterator::new(code.chars()).peekable())?;
    let (code, entry_point) = compile_toplevel(&st)?;
    run_vm(&code, entry_point).map(|v| (*v).clone())
}
