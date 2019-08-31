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

use rul::{parse_compile_run, Function, Value};
use std::rc::Rc;

#[test]
fn test_iks_basic() {
    assert_eq!(
        parse_compile_run(&"```skss").unwrap(),
        Value::Function(Function::S)
    );
    assert_eq!(
        parse_compile_run(&"`ii").unwrap(),
        Value::Function(Function::I)
    );
    assert_eq!(
        parse_compile_run(&"``ksi").unwrap(),
        Value::Function(Function::S)
    )
}

#[test]
fn test_d_promise() {
    let suspended = parse_compile_run(&"`d`ir").unwrap();
    match suspended {
        Value::Function(Function::D1(_)) => (),
        _ => panic!("expected promise"),
    }
}

#[test]
fn test_force_promise() {
    assert_eq!(
        parse_compile_run(&"``d`iri").unwrap(),
        Value::Function(Function::I)
    );
    assert_eq!(
        parse_compile_run(&"``d```skssi").unwrap(),
        Value::Function(Function::S1(Rc::new(Value::Function(Function::I))))
    );
}
