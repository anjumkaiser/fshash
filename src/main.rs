use sha2::{Digest, Sha256};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::{fs, io};

fn main() -> std::io::Result<()> {
    let exclude_list = populate_exclude_list();

    let start_path = Path::new(".");
    let _ = traverse_directory(start_path, &exclude_list);

    Ok(())
}

fn process_file(file_path: &Path) {
    let mut buffer = [0; 4096];
    let mut hasher = Sha256::new();

    let sleep_duration = std::time::Duration::from_millis(1);

    let file_name = file_path.to_path_buf();

    let mut file_handle = match std::fs::File::open(&file_path) {
        Ok(x) => x,
        Err(e) => {
            println!("error processing file [{}]", e);
            return;
        }
    };

    let file_metadata = match file_handle.metadata() {
        Ok(x) => x,
        Err(e) => {
            println!("error reading metadata {:?}", e);
            return;
        }
    };

    let file_total_size: usize = file_metadata.len() as usize;
    let mut total_bytes_read: usize = 0;

    loop {
        let bytes_read = match file_handle.read(&mut buffer[..]) {
            Ok(x) => x,
            Err(e) => {
                println!("error reading bytes {:?}", e);
                0
            }
        };

        if bytes_read == 0 {
            break;
        }

        total_bytes_read += bytes_read;

        if total_bytes_read < file_total_size {
            std::thread::sleep(sleep_duration);
        }

        hasher.update(&mut buffer[..bytes_read]);
    }

    let hash_result = hasher.finalize();

    println!("{:?} hashed [{:x}]", file_name, hash_result);
}

fn traverse_directory(dir: &Path, exclude_list: &[PathBuf]) -> io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    let str = dir.to_path_buf();

    println!("tranversing directory: [{:?}]", str);

    let dir_entries = match fs::read_dir(dir) {
        Ok(x) => x,
        Err(e) => {
            println!("error traversing directory [{}]", e);
            return Ok(());
        }
    };

    for dir_entry in dir_entries {
        if let Ok(dir_entry) = dir_entry {
            let path = dir_entry.path();
            if is_excluded_path(&path, exclude_list) {
                continue;
            }
            if path.is_dir() {
                let _ = traverse_directory(&path, &exclude_list);
            } else if path.is_file() {
                process_file(&path);
            }
        }
    }

    Ok(())
}

fn is_excluded_path(filename: &Path, exclude_list: &[PathBuf]) -> bool {
    for excluded_entry in exclude_list {
        if filename == excluded_entry {
            println!(
                "excluded entry matched at {:?}, {:?}, skipping ...",
                filename, excluded_entry
            );
            return true;
        }
    }
    false
}

fn populate_exclude_list() -> Vec<PathBuf> {
    let mut exclude_list: std::vec::Vec<PathBuf> = Vec::<PathBuf>::new();
    exclude_list.push(PathBuf::from("/dev"));
    exclude_list.push(PathBuf::from("/proc"));
    exclude_list.push(PathBuf::from("/tmp"));
    exclude_list.push(PathBuf::from("/var"));
    exclude_list
}
