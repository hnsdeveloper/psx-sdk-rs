use args::*;
use cargo_metadata::MetadataCommand;
use clap::Parser;
use commands::*;

mod args;
mod commands;
mod process_error;
mod xml;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Cli::parse();
    let metadata = &MetadataCommand::new()
        .exec()
        .expect("Could not parse metadata");

    if let Commands::Clean = opt.command {
        clean(&metadata)?;
        return Ok(());
    }

    let build_args = opt.command.build_args();
    let subcmd: &str = opt.command.clone().into();
    run_cargo_compile(&build_args, &subcmd)?;
    if let Commands::Check = opt.command {
        return Ok(());
    }

    if build_args.iso {
        build_iso(&metadata, &build_args)?;
    }

    // TODO: Handle running from a runner as when using cargo run or running the iso
    // file through an emulator

    Ok(())
}
