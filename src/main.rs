use std::borrow::Borrow;
use std::io::{stdin, stdout, Write};
use std::rc::Rc;

use crate::parse::{parse_toplevel, Application, CharPosIterator, Combinator, SyntaxTree};

mod parse;

#[derive(Debug, Clone, Eq, PartialEq)]
enum Value {
    Function(Function),
}

impl Value {
    pub fn from_combinator(c: Combinator) -> Self {
        Self::Function(match c {
            Combinator::I => Function::I,
            Combinator::K => Function::K,
            Combinator::S => Function::S,
            Combinator::V => Function::V,
            Combinator::D => Function::D,
            Combinator::Dot(ch) => Function::Dot(ch),
            _ => panic!("{:?} not supported.", c),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum Function {
    I,
    K,
    K1(Rc<Value>),
    S,
    S1(Rc<Value>),
    S2(Rc<Value>, Rc<Value>),
    V,
    D,
    D1(usize),
    Dot(char),
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

const K2_CODE: [OpCode; 5] = [
    OpCode::Invoke,
    OpCode::Rot,
    OpCode::Invoke,
    OpCode::Swap,
    OpCode::Invoke,
];

#[derive(Debug, Clone, Eq, PartialEq)]
struct VmState {
    stack: Vec<Rc<Value>>,
    rstack: Vec<(usize, usize)>,
    pc: usize,
}

impl Default for VmState {
    fn default() -> Self {
        Self {
            stack: Vec::new(),
            rstack: Vec::new(),
            pc: 0,
        }
    }
}

fn run_vm(code: &[OpCode], entry_point: usize) -> Result<Rc<Value>, String> {
    let mut vm_state = VmState::default();
    vm_state.pc = entry_point;
    loop {
        let opcode = code[vm_state.pc];
        match opcode {
            OpCode::Placeholder => panic!("placeholder not replaced during compilation."),
            OpCode::PushImmediate(c) => vm_state.stack.push(Rc::new(Value::from_combinator(c))),
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
                if let Value::Function(Function::D) = vm_state.stack[vm_state.stack.len() - 1].borrow() {
                    vm_state.stack.pop().unwrap();
                    vm_state.stack.push(Rc::new(Value::Function(Function::D1(vm_state.pc + 1))));
                    vm_state.pc += offset;
                }
            }
            OpCode::Invoke => invoke(code, &mut vm_state)?,
            OpCode::Finish => {
                debug_assert_eq!(vm_state.stack.len(), 1);
                return Ok(vm_state.stack.pop().unwrap());
            }
        }
        match opcode {
            OpCode::Invoke | OpCode::CheckSuspend(_) => (),
            _ => vm_state.pc += 1,
        }
        if let Some((to, auto_return)) = vm_state.rstack.get(vm_state.rstack.len() - 1) {
            if vm_state.pc == *auto_return {
                vm_state.pc = *to;
                vm_state.rstack.pop();
            }
        }
    }
}

fn invoke(code: &[OpCode], vm_state: &mut VmState) -> Result<(), String> {
    let stack = &mut vm_state.stack;
    let rstack = &mut vm_state.rstack;
    let (arg, fun) = (stack.pop().unwrap(), stack.pop().unwrap());
    match fun.borrow() {
        Value::Function(f) => match f {
            Function::I => stack.push(arg),
            Function::K => stack.push(Rc::new(Value::Function(Function::K1(arg)))),
            Function::K1(val) => stack.push(val.clone()),
            Function::S => stack.push(Rc::new(Value::Function(Function::S1(arg)))),
            Function::S1(val) => {
                stack.push(Rc::new(Value::Function(Function::S2(val.clone(), arg))))
            }
            Function::S2(val1, val2) => {
                stack.push(val1.clone());
                stack.push(arg.clone());
                stack.push(val2.clone());
                stack.push(arg.clone());
                rstack.push((vm_state.pc + 1, K2_CODE.len()));
                vm_state.pc = 0;
            }
            Function::V => stack.push(fun.clone()),
            Function::D => panic!("d operator invoked"),
            Function::D1(at) => {
                if let OpCode::CheckSuspend(offset) = code[*at - 1] {
                    rstack.push((vm_state.pc + 1, *at - 1 + offset));
                    vm_state.pc = *at;
                } else {
                    panic!("promise does not point to after a CheckSuspend opcode");
                }
            }
            Function::Dot(ch) => {
                print!("{}", ch);
                stack.push(arg);
            }
        },
    }
    match fun.borrow() {
        Value::Function(Function::S2(_, _)) => (),
        _ => vm_state.pc += 1,
    }
    Ok(())
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
    let entry_point = code.len();
    compile(st, &mut code)?;
    code.push(OpCode::Finish);
    Ok((code, entry_point))
}

fn parse_compile_run(code: &str) -> Result<Rc<Value>, String> {
    let st = parse_toplevel(&mut CharPosIterator::new(code.chars()).peekable())?;
    //let mut paren = String::new();
    //print_parenthesized(&st, 0, 0, &mut paren);
    //println!("P> {}", &paren);
    let (code, entry_point) = compile_toplevel(&st)?;
    //println!("C> {:?}, {}", &code, &entry_point);
    run_vm(&code, entry_point)
}

fn main() {
    let mut input = String::new();
    loop {
        print!(">> ");
        stdout().flush().unwrap();
        input.clear();
        stdin().read_line(&mut input).unwrap();
        if &input.trim().to_lowercase() == "exit" {
            break;
        }
        match parse_compile_run(&input) {
            Ok(v) => println!("=> {:?}", v),
            Err(e) => println!("!! {}", e),
        }
    }
}
