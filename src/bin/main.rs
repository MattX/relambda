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

use std::fs::read_to_string;
use std::io::{stdin, stdout, Write};

use clap::{crate_version, App, Arg, ArgMatches};
use log::Level;

use relambda::parse_compile_run;

fn main() -> Result<(), ()> {
    let args = get_args().ok_or(())?;
    match args.value_of("input_file") {
        Some(f) => run_file(f),
        None => repl(args.is_present("silent")),
    }
    Ok(())
}

fn repl(silent: bool) {
    let mut input = String::new();
    loop {
        if !silent {
            print!(">> ");
            stdout().flush().unwrap();
        }
        input.clear();
        let read = stdin().read_line(&mut input).unwrap();
        if read == 0 {
            return;
        }
        if &input.trim().to_lowercase() == "exit" {
            break;
        }
        match parse_compile_run(&input) {
            Ok(v) => {
                if !silent {
                    println!("=> {:?}", v)
                }
            }
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

fn get_args() -> Option<ArgMatches<'static>> {
    let matches = App::new("relambda")
        .version(crate_version!())
        .arg(Arg::with_name("input_file").help("File to execute. If not set, will start a REPL."))
        .arg(
            Arg::with_name("silent")
                .short("s")
                .long("silent")
                .help("If in REPL mode, controls whether to show prompts and return values."),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .help("Print debugging information."),
        )
        .get_matches();
    if matches.is_present("input_file") && matches.is_present("silent") {
        println!("--silent cannot be used with an input file.");
        return None;
    }
    let verbosity = if matches.is_present("verbose") {
        Level::Debug as usize
    } else {
        0
    };
    stderrlog::new().verbosity(verbosity).init().unwrap();
    Some(matches)
}
