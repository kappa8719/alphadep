use clap::Parser;
use interface::configuration::runtime::RuntimeConfiguration;
use std::fs::File;
use std::io::{Write, stderr};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::{fs, io};

#[derive(Parser, Debug)]
pub struct CommandLineArgs {
    #[arg(long = "archive.extract")]
    archive_extract: Option<String>,

    #[arg(long = "archive.extract.dest")]
    archive_extract_dest: Option<String>,

    #[arg(long = "archive.extract.overwrite", default_value_t = false)]
    archive_extract_overwrite: bool,

    #[arg(long = "build", default_value_t = false)]
    build: bool,
    #[arg(long = "execute", default_value_t = false)]
    execute: bool,
}

fn resolve_configuration() -> RuntimeConfiguration {
    let configuration = fs::read_to_string("alphadep-runtime.toml").unwrap();
    toml::from_str::<RuntimeConfiguration>(configuration.as_str()).unwrap()
}

fn main() {
    let args = CommandLineArgs::parse();
    if let Some(extract) = args.archive_extract {
        println!("archive/extract: from {extract}");
        let dest_root = match args.archive_extract_dest {
            None => PathBuf::from("."),
            Some(dest) => PathBuf::from(dest),
        };
        println!(
            "archive/extract: destination {}",
            dest_root.to_str().unwrap()
        );

        if args.archive_extract_overwrite {
            println!("archive/extract: restructuring destination root -");
            fs::remove_dir_all(dest_root.clone()).unwrap();
        }

        fs::create_dir_all(dest_root.clone()).unwrap();

        println!("archive/extract: extracting -");
        let path = PathBuf::from(extract);
        let file = File::open(path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let dest = match file.enclosed_name() {
                Some(path) => dest_root.join(path),
                None => continue,
            };

            if file.is_dir() {
                fs::create_dir_all(&dest).unwrap();
            } else {
                if let Some(p) = dest.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p).unwrap();
                    }
                }
                let mut outfile = File::create(&dest).unwrap();
                io::copy(&mut file, &mut outfile).unwrap();
            }

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&dest, fs::Permissions::from_mode(mode)).unwrap();
            }
        }

        return;
    }

    if args.build {
        let configuration = resolve_configuration();

        let Some(build_script) = configuration.build.script.clone() else {
            stderr()
                .write_all(b"build: abort + missing build script")
                .unwrap();
            return;
        };

        println!("build: begin -");

        let build_result = Command::new("sh")
            .arg("-c")
            .arg(build_script)
            .output()
            .unwrap();
        println!("build: exited {:?}", build_result.status.code(),);

        return;
    }

    if args.execute {
        let configuration = resolve_configuration();

        println!("execution: begin -");
        let exec_result = Command::new("sh")
            .arg("-c")
            .arg(configuration.execution.script.clone())
            .output()
            .unwrap();
        println!("execution: exited {:?}", exec_result.status.code());
    }
}
