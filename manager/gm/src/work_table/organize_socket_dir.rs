extern crate itertools;
extern crate ofiles;

use itertools::Itertools;
use std::fs;
use std::path;

/// 1. Deletes any left over directories from socket dir
/// 2. Deletes any left over/non-connected sockets from socket dir
///
/// Returns a list of active sockets and their PIDs
/// Returns an error string if there were other types of files in the left over socket dirs
/// or there were permission denied errors
/// or other types of read error.
pub fn organize_socket_dir(
    sockets_dir: &String,
) -> Result<Vec<(path::PathBuf, Vec<ofiles::Pid>)>, String> {
    let mut active_sockets: Vec<(path::PathBuf, Vec<ofiles::Pid>)> = Vec::new();
    let mut unexpected_files: Vec<String> = Vec::new();

    let top_dir: path::PathBuf = path::PathBuf::from(sockets_dir);

    // https://natclark.com/tutorials/rust-list-all-files/
    for entry_r in fs::read_dir(top_dir).or(Err(format!("Could not read dir {}", sockets_dir)))? {
        let entry = entry_r.or_else(|err| {
            Err(format!(
                "Could not read dir {}\n {}",
                sockets_dir,
                err.to_string()
            ))
        })?;
        let socket_dir: path::PathBuf = entry.path();
        if !socket_dir.is_dir() {
            continue;
        }
        let mut empty = true;
        let entries1 = fs::read_dir(socket_dir.clone()).or_else(|err| {
            Err(format!(
                "Could not read directory {}\n{}",
                socket_dir.as_path().display(),
                err.to_string()
            ))
        })?;
        for entry1_r in entries1 {
            let entry1 = entry1_r.or_else(|err| {
                Err(format!(
                    "Could not read directory {}\n{}",
                    socket_dir.as_path().display(),
                    err.to_string()
                ))
            })?;
            let socket: path::PathBuf = entry1.path();
            if entry1.file_name() == "PAIR.zmq" {
                // https://docs.rs/ofiles/0.2.0/ofiles/fn.opath.html
                let pids = ofiles::opath(socket_dir.clone()).or_else(|err| Err(err.to_string()))?;
                if pids.len() > 0 {
                    active_sockets.push((socket.clone(), pids));
                    empty = false;
                } else {
                    fs::remove_file(socket).or_else(|err| Err(err.to_string()))?;
                }
            } else {
                unexpected_files.push(entry1.path().display().to_string());
                empty = false;
            }
        }
        if empty {
            fs::remove_dir(socket_dir.clone()).or(Err(format!(
                "Could not remove dir {}",
                socket_dir.as_path().display()
            )))?;
        }
    }
    if unexpected_files.len() > 0 {
        let unexpected_files_s = Itertools::join(&mut unexpected_files.iter(), "\n");
        return Err(format!(
            "Unexpected files in socket dir \n{}",
            unexpected_files_s
        ));
    };
    return Ok(active_sockets);
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate tempdir;
    use std::fs;

    #[test]
    fn test_clear_empty_socket_dir() {
        use tempdir::TempDir;
        let tmp_dir = TempDir::new("test_sockets_dir").unwrap();
        let empty_socket_dir = tmp_dir.path().join("empty-socket-dir");
        fs::create_dir(empty_socket_dir.clone()).unwrap();
        print!(
            "Created empty socket dir: {}\n",
            empty_socket_dir.as_path().display()
        );
        let mut contains = false;
        for entry_r in fs::read_dir(tmp_dir.path()).unwrap() {
            let entry = entry_r.unwrap();
            print!("Found {}\n", entry.file_name().into_string().unwrap());
            if entry.file_name() == "empty-socket-dir" {
                contains = true;
            } else {
                assert!(false);
            }
        }
        assert!(contains);
        let temp_dir_path: String = tmp_dir.path().as_os_str().to_str().unwrap().to_owned();
        organize_socket_dir(&temp_dir_path).unwrap();
        assert_eq!(fs::read_dir(tmp_dir).unwrap().count(), 0);
    }

    #[test]
    fn test_fail_on_dirty_socket_dir() {
        use tempdir::TempDir;
        let tmp_dir = TempDir::new("test_sockets_dir").unwrap();
        let dirty_socket_dir = tmp_dir.path().join("dirty-socket-dir");
        fs::create_dir(dirty_socket_dir.clone()).unwrap();
        fs::File::create(dirty_socket_dir.join("mess")).unwrap();
        print!(
            "Created dirty socket dir: {}\n",
            dirty_socket_dir.as_path().display()
        );
        let mut contains = false;
        for entry_r in fs::read_dir(tmp_dir.path()).unwrap() {
            let entry = entry_r.unwrap();
            print!("Found {}\n", entry.file_name().into_string().unwrap());
            if entry.file_name() == "dirty-socket-dir" {
                contains = true;
            } else {
                assert!(false);
            }
        }
        assert!(contains);
        let temp_dir_path: String = tmp_dir.path().as_os_str().to_str().unwrap().to_owned();
        match organize_socket_dir(&temp_dir_path) {
            Ok(_) => assert!(false),
            Err(error) => assert_eq!(
                error,
                format!(
                    "Unexpected files in socket dir \n{}/mess",
                    dirty_socket_dir.as_path().display()
                )
            ),
        }
    }
}
