use cargo_metadata::MetadataCommand;
use clap::{Args, Parser, Subcommand};
use std::env;
use std::process::{self, Command, Stdio};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long)]
    toolchain: Option<String>,

    #[arg(short, long)]
    cargo_args: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    Build(BuildArgs),
    Run(BuildArgs),
    Check,
    Clean,
}

impl From<Commands> for &'static str {
    fn from(cmd: Commands) -> &'static str {
        match cmd {
            Commands::Build(_) => "build",
            Commands::Run(_) => "run",
            Commands::Check => "check",
            Commands::Clean => panic!(),
        }
    }
}

#[derive(Args, Default, Clone)]
struct BuildArgs {
    #[arg(long)]
    link: Option<String>,
    #[arg(long)]
    features: Option<String>,
    #[arg(long)]
    load_offset: Option<u32>,
    #[arg(long)]
    stack_pointer: Option<u32>,
    #[arg(long)]
    elf: bool,
    #[arg(long)]
    debug: bool,
    #[arg(long)]
    lto: bool,
    #[arg(long)]
    small: bool,
}

const CARGO_CMD: &str = "cargo";

fn main() {
    let opt = Cli::parse();

    let mut cargo_args: Vec<String> = opt
        .cargo_args
        .iter()
        .map(|arg| {
            let mut s = arg.to_string();
            s.insert_str(0, "--");
            s.split(' ').map(|s| s.to_string()).collect::<Vec<String>>()
        })
        .flatten()
        .collect();

    // Always compile in release mode
    cargo_args.push("--release".to_string());

    // Set toolchain if not default
    let toolchain = match opt.toolchain {
        Some(name) => format!("+{}", name),
        None => "+nightly".to_string(),
    };

    // Set build-std option to pass to cargo
    let build_std = "-Zbuild-std=core,alloc".to_string();

    // Rust doesn't do cross-crate inlining unless functions are marked as
    // #[inline]. Pretty much everything in the psx crate should be inlined since
    // they're such low-level functions, but to avoid doing that manually we
    // codegen-units to 1 by default to get essentially the same effect without the
    // burden of always doing LTO. This default is overriden when setting RUSTFLAGS
    // through an env var, but the performance of builds without this flag is
    // extremely unpredictable.
    let default_rustflags = "-Ccodegen-units=1".to_string();
    // Try getting RUSTFLAGS from env
    let mut rustflags = env::var("RUSTFLAGS").ok().unwrap_or(default_rustflags);

    let build_args = match &opt.command {
        Commands::Build(build_args) | Commands::Run(build_args) => build_args.clone(),
        Commands::Check => BuildArgs::default(),
        Commands::Clean => BuildArgs::default(),
    };

    let script = build_args.link.unwrap_or("psexe.ld".to_string());
    // Set linker script if any
    rustflags.push_str(&format!(" -Clink-arg=-T{}", script));
    let format = if build_args.debug || build_args.elf {
        "elf32-tradlittlemips"
    } else {
        "binary"
    };
    rustflags.push_str(&format!(" -Clink-arg=--oformat={}", format));

    // Set optional RUSTFLAGS
    if build_args.debug {
        rustflags.push_str(" -g");
    }

    if build_args.lto {
        rustflags.push_str(" -Clto=fat -Cembed-bitcode=yes");
    }

    if build_args.small {
        rustflags.push_str(" -Copt-level=s");
    }

    let metadata = &MetadataCommand::new()
        .exec()
        .expect("Could not parse metadata");

    if let Commands::Clean = opt.command {
        for pkg in &metadata.packages {
            let mut clean = Command::new(CARGO_CMD)
                .arg("clean")
                .arg("-p")
                .arg(&pkg.name)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("`cargo clean` failed to start");
            let status = clean.wait().expect("`cargo clean` wasn't running");
            if !status.success() {
                let code = status.code().unwrap_or(1);
                process::exit(code);
            }
            return;
        }
    }

    let subcmd: &str = opt.command.into();
    let mut cmd = Command::new(CARGO_CMD);
    cmd.arg(toolchain)
        .arg(subcmd)
        .arg(build_std)
        .arg("-Zbuild-std-features=compiler-builtins-mem")
        .arg("--target")
        .arg("mipsel-sony-psx")
        .args(cargo_args)
        .env("RUSTFLAGS", rustflags);
    if let Some(features) = build_args.features {
        cmd.arg("--features").arg(features);
    }
    if let Some(offset) = build_args.load_offset {
        assert!(offset % 4 == 0, "Load offset must be a multiple of 4 bytes");
        cmd.env("PSX_LOAD_OFFSET", offset.to_string());
    }
    if let Some(sp) = build_args.stack_pointer {
        assert!(
            sp % 4 == 0,
            "Initial stack pointer must be a multiple of 4 bytes"
        );
        cmd.env("PSX_STACK_POINTER", sp.to_string());
    }
    let mut build = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect(&format!("`cargo {:?}` failed to start", subcmd));
    let status = build.wait().expect("`cargo build` wasn't running");
    if !status.success() {
        let code = status.code().unwrap_or(1);
        process::exit(code);
    }
}
