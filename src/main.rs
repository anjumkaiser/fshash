use std::path::{Path, PathBuf};
use std::{fs, io};

fn main() -> std::io::Result<()> {
    let exclude_list = populate_exclude_list();

    let start = Path::new("/tmp");
    let _ = traverse_directory(start, &exclude_list);

    //visit_dir();

    Ok(())
}

fn process_file(file: &Path) {
    let file_name = file.to_path_buf();
    let file_metadata = file.metadata();
    println!(
        "processing file: [{:?}], metadata [{:?}]",
        file_name, file_metadata
    );
}

fn traverse_directory(dir: &Path, exclude_list: &Vec<PathBuf>) -> io::Result<()> {
    if dir.is_dir() == false {
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
            if path.is_dir() {
                let _ = traverse_directory(&path, &exclude_list);
            } else if path.is_file() {
                process_file(&path);
            }
        }
    }

    //dbg!(result);

    Ok(())
}

fn populate_exclude_list() -> Vec<PathBuf> {
    let mut exclude_list: std::vec::Vec<PathBuf> = Vec::<PathBuf>::new();
    exclude_list.push(PathBuf::from("/dev"));
    exclude_list.push(PathBuf::from("/proc"));
    exclude_list.push(PathBuf::from("/tmp"));
    exclude_list.push(PathBuf::from("/var"));
    exclude_list
}
