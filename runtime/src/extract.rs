use crate::Instance;
use crate::command::ExtractArgs;
use log::info;
use std::fs::File;
use std::os::unix::prelude::PermissionsExt;
use std::path::PathBuf;
use std::{fs, io};

pub fn extract(_: &mut Instance, args: ExtractArgs) {
    println!("archive/extract: from {:?}", args.archive);
    let dest_root = match args.destination {
        None => PathBuf::from("."),
        Some(dest) => PathBuf::from(dest),
    };
    info!(
        "archive/extract: destination {}",
        dest_root.to_str().unwrap()
    );

    if args.overwrite {
        info!("archive/extract: recreating destination root -");
        fs::remove_dir_all(dest_root.clone()).unwrap();
    }

    fs::create_dir_all(dest_root.clone()).unwrap();

    info!("archive/extract: extracting -");
    let path = PathBuf::from(args.archive);
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
}
