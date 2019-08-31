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

use std::env::args;
use std::fs::read_to_string;
use std::io::{stdin, stdout, Write};

use rul::parse_compile_run;

fn main() {
    let args: Vec<_> = args().collect();
    match args.len() {
        1 => repl(),
        2 => run_file(&args[1]),
        _ => println!("usage: {} [filename]", &args[0]),
    }
}

fn repl() {
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

fn run_file(fname: &str) {
    let contents = read_to_string(fname).unwrap();
    match parse_compile_run(&contents) {
        Ok(_) => (),
        Err(e) => println!("Error: {}", e),
    }
}
