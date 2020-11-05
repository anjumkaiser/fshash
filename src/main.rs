use sha2::{Digest, Sha256};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::{fs, io};

fn main() -> std::io::Result<()> {
    let project_base_dir = match directories_next::ProjectDirs::from("com", "fshash", "fshash") {
        Some(project_base_dir) => project_base_dir,
        None => {
            println!("FATAL ERROR: unable to get base directory");
            std::process::exit(0);
        }
    };

    let project_data_local_dir = project_base_dir.data_local_dir();

    println!("a {:?}", project_data_local_dir.to_str().unwrap());

    let mut sqlite_store_path = PathBuf::from(project_data_local_dir.to_str().unwrap());
    sqlite_store_path.push("data.sql");
    println!("sqlite data store path [{:?}]", sqlite_store_path);

    let sqlite_connection = match sqlite::open(&sqlite_store_path) {
        Ok(x) => x,
        Err(_) => {
            if let Err(e) = std::fs::metadata(project_data_local_dir) {
                println!("{:?}", e);

                println!("creating directory ...");
                if let Err(e) = std::fs::create_dir_all(project_data_local_dir) {
                    println!("failed to create directory {:?}, aborting ..", e);
                    std::process::exit(0);
                }
            }

            let sqlite_connection = match sqlite::open(sqlite_store_path) {
                Ok(x) => x,
                Err(e) => {
                    println!("error processing file [{}]", e);
                    std::process::exit(0);
                }
            };

            if let Err(e) = sqlite_connection.execute(
                "CREATE TABLE file_hashes ( path TEXT NOT NULL PRIMARY KEY, hash TEXT NOT NULL, date_modified int NOT NULL);",
            ) {

                println!("error creating table [{:?}]", e );
                std::process::exit(0);

            }
            sqlite_connection
        }
    };

    let exclude_list = populate_exclude_list(&[&project_data_local_dir]);

    let start_path = Path::new(".");
    let _ = traverse_directory(start_path, &exclude_list, &sqlite_connection);

    Ok(())
}

fn process_file(file_path: &Path, sqlite_connection: &sqlite::Connection) {
    let mut buffer = [0; 4096];
    let mut hasher = Sha256::new();

    let mut hash_this_file: bool = true;

    let sleep_duration = std::time::Duration::from_millis(1);

    let file_name = file_path.to_path_buf();
    let file_metadata = match file_path.metadata() {
        Ok(x) => x,
        Err(e) => {
            println!("error getting file metadata, skipping [{}]", e);
            return;
        }
    };

    let file_sys_date_modified = match file_metadata.modified() {
        Ok(x) => x,
        Err(e) => {
            println!("unable to get mdoified time of file, skipping . {:?}", e);
            return;
        }
    };

    let file_sys_date_modified_since_epoh: std::time::Duration =
        match file_sys_date_modified.duration_since(std::time::SystemTime::UNIX_EPOCH) {
            Ok(x) => x,
            Err(e) => {
                println!("FATAL Error converting time, [{:?}] skipping ...", e);
                return;
            }
        };

    let mut statement = match sqlite_connection.prepare("SELECT * FROM file_hashes WHERE path = ?")
    {
        Ok(x) => x,
        Err(e) => {
            println!("unable to prepare statement {:?}", e);
            return;
        }
    };

    let fname_str = match file_name.to_str() {
        Some(x) => x,
        None => {
            return;
        }
    };

    let _ = statement.bind(1, fname_str);

    while let sqlite::State::Row = match statement.next() {
        Ok(x) => x,
        Err(_) => {
            println!("FATAL: unable to fetch row, skipping...");
            return;
        }
    } {
        if let Ok(x) = statement.read::<f64>(2) {
            if x == file_sys_date_modified_since_epoh.as_secs_f64() {
                hash_this_file = false;
            }
        };
    }

    if hash_this_file == false {
        println!("skipping file {:?}", file_name);
        return;
    }

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

    let query =
        format!(
        "insert or replace into file_hashes(path, hash, date_modified) values ({:?}, '{:x}', '{}')",
        file_name, hash_result, file_sys_date_modified_since_epoh.as_secs_f64()
    );

    //println!("query to execute {}", query);

    if let Err(e) = sqlite_connection.execute(query) {
        println!("error inserting into table {:?}", e);
    };
}

fn traverse_directory(
    dir: &Path,
    exclude_list: &[PathBuf],
    sqlite_connection: &sqlite::Connection,
) -> io::Result<()> {
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
                let _ = traverse_directory(&path, &exclude_list, &sqlite_connection);
            } else if path.is_file() {
                process_file(&path, &sqlite_connection);
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

fn populate_exclude_list(list: &[&Path]) -> Vec<PathBuf> {
    let mut exclude_list: std::vec::Vec<PathBuf> = Vec::<PathBuf>::new();

    for item in list {
        let p = item as &Path;
        exclude_list.push(p.to_path_buf());
    }

    exclude_list.push(PathBuf::from("/dev"));
    exclude_list.push(PathBuf::from("/proc"));
    exclude_list.push(PathBuf::from("/tmp"));
    exclude_list.push(PathBuf::from("/var"));

    exclude_list.sort();

    exclude_list
}
