use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    Build(BuildArgs),
    Run(BuildArgs),
    Check,
    Clean,
}

impl Commands {
    pub fn build_args(&self) -> BuildArgs {
        match &self {
            Commands::Build(a) | Commands::Run(a) => a.clone(),
            _ => BuildArgs::default(),
        }
    }
}

impl From<Commands> for &'static str {
    fn from(cmd: Commands) -> &'static str {
        match cmd {
            Commands::Build(_) => "build",
            Commands::Run(_) => "build",
            Commands::Check => "check",
            Commands::Clean => panic!(),
        }
    }
}

#[derive(Args, Default, Clone)]
pub struct BuildArgs {
    #[arg(long)]
    pub link: Option<String>,
    #[arg(long)]
    pub features: Option<String>,
    #[arg(long)]
    pub load_offset: Option<u32>,
    #[arg(long)]
    pub stack_pointer: Option<u32>,
    #[arg(long)]
    pub elf: bool,
    #[arg(long)]
    pub debug: bool,
    #[arg(long)]
    pub lto: bool,
    #[arg(long)]
    pub small: bool,
    #[arg(long, conflicts_with = "elf", conflicts_with = "debug")]
    pub iso: bool,
    #[arg(long, requires = "iso")]
    pub xml: Option<String>,
    #[arg(long, requires = "iso", conflicts_with = "xml")]
    pub path: Option<String>,
    #[arg(long, requires = "iso", conflicts_with = "xml")]
    pub appid: Option<String>,
    #[arg(long, requires = "iso", conflicts_with = "xml")]
    pub volume: Option<String>,
    #[arg(long, requires = "iso", conflicts_with = "xml")]
    pub publisher: Option<String>,
    #[arg(short, long)]
    pub toolchain: Option<String>,
    #[arg(short, long)]
    pub cargo_args: Vec<String>,
}
