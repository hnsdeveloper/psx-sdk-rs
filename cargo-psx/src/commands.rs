use crate::process_error::ProcessError;
use crate::xml;
use crate::BuildArgs;
use cargo_metadata::Metadata;
use std::{env, fs,
          fs::File,
          io::{BufWriter, Write},
          process::{self, Command, Stdio}};

const CARGO_CMD: &str = "cargo";

fn write_system_cnf(app_id: &str) -> Result<(), std::io::Error> {
    let s = format!(
        "BOOT=cdrom:\\{}.EXE;1\r\nTCB=4\r\nEVENT=16\r\nSTACK=801FFFF0\r\n",
        app_id.to_uppercase()
    );
    BufWriter::new(File::create("SYSTEM.CNF")?).write(s.as_bytes())?;
    Ok(())
}

pub fn build_iso(
    metadata: &cargo_metadata::Metadata, build_args: &BuildArgs,
) -> Result<(), std::io::Error> {
    // This seems a little bit flaky for me, but it will suffice for now. There are
    // numerous circumstances where it would break. TODO: Think of a better
    // solution
    let target = format!(
        "{}/mipsel-sony-psx/release/{}.exe",
        metadata.target_directory.as_str(),
        metadata.packages[0].name
    );

    let image_name = metadata.root_package().unwrap().name.clone();
    let app_id = if let Some(v) = build_args.appid.as_ref() {
        v.clone()
    } else {
        image_name.clone()
    };

    let xml_path = if let Some(s) = build_args.xml.as_ref() {
        s.clone()
    } else {
        let path = if let Some(p) = build_args.path.as_ref() {
            p.clone()
        } else {
            fs::create_dir_all("assets")?;
            "assets".into()
        };

        let xml = xml::generate_xml(
            &path,
            &image_name,
            &app_id,
            &target,
            build_args.volume.as_ref(),
            build_args.publisher.as_ref(),
        )?;
        let file_name = format!("{}.xml", &image_name);
        BufWriter::new(File::create(&file_name)?).write(&xml)?;
        file_name
    };

    write_system_cnf(&app_id)?;

    let mut mkpsxiso_cmd = Command::new("mkpsxiso");
    mkpsxiso_cmd
        .arg(xml_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit());
    let mut child = mkpsxiso_cmd.spawn().expect("mkpsxiso failed to start");
    let exit_status = child.wait()?;
    if !exit_status.success() {
        process::exit(exit_status.code().unwrap_or(1));
    }

    Ok(())
}

pub fn build_cargo_command(build_args: &BuildArgs, subcmd: &str) -> Command {
    let cargo_args = build_cargo_args(&build_args.cargo_args);
    let rustflags = build_rustflags(&build_args);

    // Set toolchain if not default
    let toolchain = match build_args.toolchain.as_ref() {
        Some(name) => format!("+{}", name),
        None => "+nightly".to_string(),
    };

    let mut cmd = Command::new(CARGO_CMD);
    cmd.arg(toolchain)
        .arg(subcmd)
        // Set build-std option to pass to cargo
        .arg("-Zbuild-std=core,alloc")
        .arg("-Zbuild-std-features=compiler-builtins-mem")
        .arg("--target")
        .arg("mipsel-sony-psx")
        .args(cargo_args)
        .env("RUSTFLAGS", rustflags);
    if let Some(features) = build_args.features.as_ref() {
        cmd.arg("--features").arg(features.clone());
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

    cmd
}

fn build_rustflags(build_args: &BuildArgs) -> String {
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

    let script = if let Some(v) = build_args.link.as_ref() {
        v.clone()
    } else {
        "psexe.ld".into()
    };
    //Set linker script if any
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

    rustflags
}

fn build_cargo_args(cargo_args: &Vec<String>) -> Vec<String> {
    let mut cargo_args: Vec<String> = cargo_args
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
    cargo_args
}

// Given that cargo check also compiles (but doesn't do code generation), I
// think it is a fair name for this function
pub fn run_cargo_compile(build_args: &BuildArgs, subcmd: &str) -> Result<(), ProcessError> {
    let mut cmd = build_cargo_command(&build_args, subcmd);
    let mut child = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect(&format!("`cargo {:?}` failed to start", subcmd));
    let status = child.wait().expect("`cargo build` wasn't running");
    if !status.success() {
        let args: Option<String> = if cmd.get_args().count() > 0 {
            Some(cmd.get_args().fold(String::new(), |mut acc, arg| {
                acc.push_str(arg.to_str().unwrap());
                acc
            }))
        } else {
            None
        };
        return Err(ProcessError::new(
            cmd.get_program().to_str().unwrap().to_string(),
            args,
            status.code().unwrap(),
        ));
    }
    Ok(())
}

pub fn clean(metadata: &Metadata) -> Result<(), ProcessError> {
    for pkg in &metadata.packages {
        let mut clean = Command::new(CARGO_CMD);
        clean
            .arg("clean")
            .arg("-p")
            .arg(&pkg.name)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        let mut child = clean.spawn().expect("`cargo clean` failed to start");
        let status = child.wait().expect("`cargo clean` wasn't running");
        if !status.success() {
            let args: Option<String> = if clean.get_args().count() > 0 {
                Some(clean.get_args().fold(String::new(), |mut acc, arg| {
                    acc.push_str(arg.to_str().unwrap());
                    acc
                }))
            } else {
                None
            };
            return Err(ProcessError::new(
                clean.get_program().to_str().unwrap().to_string(),
                args,
                status.code().unwrap(),
            ));
        }
    }
    Ok(())
}
