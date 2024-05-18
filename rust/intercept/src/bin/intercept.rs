/*  Copyright (C) 2012-2024 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

extern crate core;

use std::io::Write;

use anyhow::Result;
use clap::{arg, ArgAction, command};
use crossbeam_channel::bounded;

use intercept::ipc::{Envelope, Event, ReporterId};
use intercept::collector::{EventCollector, EventCollectorOnTcp};

#[derive(Debug, PartialEq)]
struct Arguments {
    command: Vec<String>,
    output: String,
    config: Option<String>,
    verbose: u8,
}

impl Arguments {
    fn parse() -> Result<Self> {
        let matches = command!()
            .args(&[
                arg!(<COMMAND> "Build command")
                    .action(ArgAction::Append)
                    .value_terminator("--")
                    .num_args(1..)
                    .last(true)
                    .required(true),
                arg!(-o --output <FILE> "Path of the result file")
                    .default_value("events.json")
                    .hide_default_value(false),
                arg!(-c --config <FILE> "Path of the config file"),
                arg!(-v --verbose ... "Sets the level of verbosity")
                    .action(ArgAction::Count),
            ])
            .get_matches();

        let result = Arguments {
            command: matches.get_many("COMMAND")
                .expect("command is required")
                .map(String::to_string)
                .collect(),
            output: matches.get_one::<String>("output")
                .expect("output is defaulted")
                .clone(),
            config: matches.get_one::<String>("config")
                .map(String::to_string),
            verbose: matches.get_count("verbose"),
        };

        Ok(result)
    }
}

fn run() -> Result<i32> {
    let arguments = Arguments::parse()?;

    // let collector = EventCollectorOnTcp::new()?;
    // let destination = collector.address()?;
    //
    // std::env::set_var("INTERCEPT_REPORT_DESTINATION", &destination.0);
    // std::env::set_var("INTERCEPT_VERBOSE", arguments.verbose.to_string());
    // let mut build = std::process::Command::new(arguments.command[0].clone())
    //     .args(&arguments.command[1..])
    //     .envs(std::env::vars())
    //     .spawn()?;
    //
    // let (sender, mut receiver) = bounded::<Envelope>(10);
    // let collector_loop = std::thread::spawn(move || {
    //     collector.collect(sender)
    // });
    // let writer_loop = std::thread::spawn(move || {
    //     let mut writer = std::fs::File::create(arguments.output)?;
    //     loop {
    //         let envelope = receiver.recv()?;
    //         let _ = envelope.write_into(&mut writer)?;
    //         writer.flush()?;
    //     }
    // });
    //
    // let build_status = build.wait()?;
    // collector.stop()?;
    //
    // collector_loop.join().unwrap()?;
    // writer_loop.join().unwrap()?;
    //
    // Ok(build_status.code().unwrap())
    Ok(0)
}

fn main() {
    let exit_code = run().unwrap_or_else(|error| {
        eprintln!("Error: {}", error);
        1
    });

    std::process::exit(exit_code);
}
