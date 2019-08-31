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

use unicode_reader::CodePoints;

use crate::parse::{parse_toplevel, Application, CharPosIterator, Combinator, SyntaxTree};

mod parse;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Function {
    I,
    K,
    K1(Rc<Function>),
    S,
    S1(Rc<Function>),
    S2(Rc<Function>, Rc<Function>),
    V,
    D,
    D1(usize),
    C,
    C1(Box<VmState>),
    E,
    Read,
    Reprint,
    Compare(char),
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum OpCode {
    Placeholder,
    PushImmediate(Combinator),
    Swap,
    Rot,
    CheckSuspend(usize),
    Invoke,
    Finish,
}

const K2_START: usize = 0;
const K2_LEN: usize = 5;
const K2_END: usize = K2_START + K2_LEN;
const K2_CODE: [OpCode; K2_LEN] = [
    OpCode::Invoke,
    OpCode::Rot,
    OpCode::Invoke,
    OpCode::Swap,
    OpCode::Invoke,
];

const D1_START: usize = K2_END;
const D1_LEN: usize = 2;
const D1_END: usize = D1_START + D1_LEN;
const D1_CODE: [OpCode; D1_LEN] = [OpCode::Swap, OpCode::Invoke];

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
            OpCode::Placeholder => panic!("placeholder not replaced during compilation."),
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
                    vm_state.stack.push(Rc::new(Function::D1(vm_state.pc + 1)));
                    vm_state.pc += offset;
                } else {
                    vm_state.pc += 1;
                }
            }
            OpCode::Invoke => {
                if let Some(ret) = invoke(code, &mut vm_state)? {
                    return Ok(ret);
                }
            }
            OpCode::Finish => {
                debug_assert_eq!(vm_state.stack.len(), 1);
                // The rstack should contain our sentinel return point
                debug_assert_eq!(vm_state.rstack, [(code.len(), code.len())]);
                return Ok(vm_state.stack.pop().unwrap());
            }
        }
        match opcode {
            OpCode::Invoke | OpCode::CheckSuspend(_) => (),
            _ => vm_state.pc += 1,
        }
        println!("{:?} ({:?} → {:?})", &vm_state, opcode, code[vm_state.pc]);

        let (to, from) = vm_state.rstack[vm_state.rstack.len() - 1];
        if vm_state.pc == from {
            println!("Jumping down {} → {}", vm_state.pc, to);
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
            vm_state.stack.push(val1.clone());
            vm_state.stack.push(arg.clone());
            vm_state.stack.push(val2.clone());
            vm_state.stack.push(arg.clone());
            vm_state.push_rstack(vm_state.pc + 1, K2_END);
            vm_state.pc = K2_START;
        }
        Function::V => vm_state.stack.push(fun.clone()),
        Function::D => panic!("d operator invoked"),
        Function::D1(at) => {
            if let OpCode::CheckSuspend(offset) = code[*at - 1] {
                vm_state.stack.push(arg);
                vm_state.push_rstack(vm_state.pc + 1, D1_END);
                vm_state.push_rstack(D1_START, *at - 2 + offset);
                vm_state.pc = *at;
            } else {
                panic!("promise does not point to a CheckSuspend opcode");
            }
        }
        Function::C => {
            let saved_state = vm_state.clone();
            vm_state.stack.push(arg);
            vm_state
                .stack
                .push(Rc::new(Function::C1(Box::new(saved_state))));

            // We now want to invoke the arg with the newly-created C1. It is guaranteed that
            // the instruction under the program counter is Invoke, so we can just avoid
            // advancing the PC.
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
            let fun = vm_state.cur_char.map_or(Function::V, |c| Function::Dot(c));
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
        Function::S2(_, _) | Function::D1(_) => (),
        // The following functions do not advance the pc in order to call OpCode::Invoke again
        Function::C | Function::Read | Function::Reprint => {
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
    let mut code = K2_CODE.to_vec();
    code.extend_from_slice(&D1_CODE);
    let entry_point = code.len();
    compile(st, &mut code)?;
    code.push(OpCode::Finish);
    println!("Compiled: {:?}", code.iter().enumerate().collect::<Vec<_>>());
    Ok((code, entry_point))
}

pub fn parse_compile_run(code: &str) -> Result<Function, String> {
    let st = parse_toplevel(&mut CharPosIterator::new(code.chars()).peekable())?;
    let (code, entry_point) = compile_toplevel(&st)?;
    run_vm(&code, entry_point).map(|v| (*v).clone())
}
