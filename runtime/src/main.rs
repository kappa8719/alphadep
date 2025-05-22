#![feature(linux_pidfd)]

mod build;
mod command;
mod manifest;
mod execute;
mod extract;

use crate::build::build;
use crate::command::{CommandLineArgs, Commands};
use crate::execute::execute;
use crate::extract::extract;
use clap::{Args, Parser, Subcommand};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::prelude::CommandExt;

#[derive(Default)]
struct Instance;

fn main() {
    colog::init();
    let mut instance = Instance::default();

    let args = CommandLineArgs::parse();

    match args.command {
        Commands::Extract(args) => extract(&mut instance, args),
        Commands::Build => build(&mut instance),
        Commands::Execute(args) => execute(&mut instance, args),
    }
}
