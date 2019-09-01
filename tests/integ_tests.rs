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

use std::rc::Rc;

use lazy_static::{initialize, lazy_static};
use log::Level;

use rul::{parse_compile_run, Expression, Function};

lazy_static! {
    static ref LOGGER: () = {
        stderrlog::new()
            .verbosity(Level::Info as usize)
            .init()
            .unwrap();
    };
}

fn setup_logging() {
    initialize(&LOGGER);
}

#[test]
fn test_iks_basic() {
    setup_logging();
    assert_eq!(parse_compile_run(&"```skss").unwrap(), Function::S);
    assert_eq!(parse_compile_run(&"`ii").unwrap(), Function::I);
    assert_eq!(parse_compile_run(&"``ksi").unwrap(), Function::S)
}

#[test]
fn test_d_promise() {
    setup_logging();
    let suspended = parse_compile_run(&"`d`ir").unwrap();
    match suspended {
        Function::D1(_) => (),
        _ => panic!("expected promise"),
    }
}

#[test]
fn test_force_promise() {
    setup_logging();
    assert_eq!(parse_compile_run(&"``d`iri").unwrap(), Function::I);
    assert_eq!(
        parse_compile_run(&"``d```skssi").unwrap(),
        Function::S1(Rc::new(Function::I))
    );
}

#[test]
fn test_call_cc() {
    setup_logging();
    assert_eq!(parse_compile_run(&"``cii").unwrap(), Function::I);
    assert_eq!(parse_compile_run(&"``cir").unwrap(), Function::Dot('\n'));
    assert_eq!(parse_compile_run(&"`c``s`kr``si`ki").unwrap(), Function::I);
}

#[test]
fn test_iv_boolean() {
    setup_logging();
    assert_eq!(
        parse_compile_run(&"`````s`kc``s`k`s`k`k`ki``ss`k`kkiks").unwrap(),
        Function::K
    );
    assert_eq!(
        parse_compile_run(&"`````s`kc``s`k`s`k`k`ki``ss`k`kkvks").unwrap(),
        Function::S
    );
}

#[test]
fn test_invoke_d() {
    setup_logging();
    assert_eq!(
        parse_compile_run(&"```sddk").unwrap(),
        Function::K1(Rc::new(Function::D1(Expression::Function(Rc::new(
            Function::K
        )))))
    );
}
