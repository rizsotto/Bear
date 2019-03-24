/*  Copyright (C) 2012-2018 by László Nagy
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

use std::path;

use Result;
use intercept::{Execution, ExitCode, InterceptModes, Session};

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Supervise {
        session: Session,
        execution: Execution,
    },
    InjectWrappers {
        command: Vec<String>,
        modes: InterceptModes,
    },
    OntologyBuild {
        output: path::PathBuf,
        command: Vec<String>,
        modes: InterceptModes,
    },
    CompilationDatabaseBuild {
        output: path::PathBuf,
        command: Vec<String>,
        modes: InterceptModes,
        config: path::PathBuf,
    },
}

impl Command {

    pub fn run(self) -> Result<ExitCode> {

//        let config = Config::default();
//        let target =
//            JsonCompilationDatabase::new(
//                path::Path::new("./compile_commands.json"));
//        let builder = Builder::new(&config, &target);
//
//        intercept_build(&builder, command.as_ref())
        unimplemented!()
    }

//fn intercept_build(builder: &Builder, command: &[String]) -> Result<ExitCode> {
//    let collector = protocol::collector::Protocol::new()
//        .chain_err(|| "Failed to set up event collection.")?;
//
//    let exit = run_build(command, collector.path())
//        .chain_err(|| "Failed to run the build.")?;
//
//    builder.build(collector.events())
//        .chain_err(|| "Failed to write output.")?;
//
//    Ok(exit)
//}
//
//fn run_build(command: &[String], destination: &path::Path) -> Result<ExitCode> {
//    env::set_var(KEY_DESTINATION, destination);
//
//    let mut sender = protocol::sender::Protocol::new(destination)?;
//    let mut build = Supervisor::new(|event| sender.send(event));
//    let exit = build.run(command)?;
//    info!("Build finished with status code: {}", exit);
//    Ok(exit)
//}
}
