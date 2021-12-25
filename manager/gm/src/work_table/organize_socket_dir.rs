extern crate itertools;
extern crate ofiles;

use super::test_channels;
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
    let entries = fs::read_dir(top_dir).or_else(|err| {
        Err(format!(
            "Could not read dir {} \n {}",
            sockets_dir,
            err.to_string()
        ))
    })?;
    for entry_r in entries {
        // Not sure why DirEntries are wrapped in a Result
        // https://www.gnu.org/software/libc/manual/html_mono/libc.html#Reading_002fClosing-Directory
        // The only possible error here from glibc's standpoint is EBADF which is irrelivant as we just got
        // a valid FD from glibc.
        let entry = entry_r.unwrap();
        let report_str = format!("Reading first level dir entry. {}", entry.path().to_string_lossy());
        test_channels::report(&report_str);
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
            test_channels::report("Reading second level dir entry.");
            let entry1 = entry1_r.unwrap();
            let socket: path::PathBuf = entry1.path();
            if entry1.file_name() == "PAIR.zmq" {
                // https://docs.rs/ofiles/0.2.0/ofiles/fn.opath.html
                let pids = ofiles::opath(socket_dir.clone()).or_else(|err| {
                    Err(format!(
                        "Error looking up socket information for socket {}\n{}.",
                        socket.to_string_lossy(),
                        err.to_string()
                    ))
                })?;
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
            fs::remove_dir(socket_dir.clone()).or_else(|err| {
                Err(format!(
                    "Could not remove dir {}\n{}",
                    socket_dir.as_path().display(),
                    err.to_string(),
                ))
            })?;
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
        let tmp_dir = TempDir::new("test_sockets_dir1").unwrap();
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
                unreachable!();
            }
        }
        assert!(contains);
        let temp_dir_path: String = tmp_dir.path().as_os_str().to_str().unwrap().to_owned();
        match organize_socket_dir(&temp_dir_path) {
            Ok(dirs) => assert_eq!(dirs.len(), 0),
            Err(_) => unreachable!(),
        };
        assert_eq!(fs::read_dir(&tmp_dir).unwrap().count(), 0);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn test_unreadable_sockets_dir() {
        use std::fs::set_permissions;
        use std::fs::Permissions;
        use std::os::unix::fs::PermissionsExt;
        use tempdir::TempDir;
        let tmp_dir = TempDir::new("test_sockets_dir2").unwrap();
        let no_permissions: Permissions = Permissions::from_mode(0o000);
        set_permissions(tmp_dir.path(), no_permissions).unwrap();
        let temp_dir_path: String = tmp_dir.path().as_os_str().to_str().unwrap().to_owned();
        match organize_socket_dir(&temp_dir_path) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e,
                format!(
                    "Could not read dir {} \n Permission denied (os error 13)",
                    temp_dir_path
                )
            ),
        };
        // re-enable writing so we can clean up.
        let normal_permissions: Permissions = Permissions::from_mode(0o644);
        set_permissions(tmp_dir.path(), normal_permissions).unwrap();
        assert_eq!(fs::read_dir(&tmp_dir).unwrap().count(), 0);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn test_interrupted_socket_dir_listing() {
        use tempdir::TempDir;
        use test_channels::for_tests::*;
        let tmp_dir = TempDir::new("test_sockets_dir3").unwrap();
        let socket_dir = tmp_dir.path().join("socket-dir");
        fs::create_dir(socket_dir.clone()).unwrap();
        fs::write(socket_dir.join("PAIR.zmq"), "foo").unwrap();
        let temp_dir_path: String = tmp_dir.path().as_os_str().to_str().unwrap().to_owned();
        use std::thread;
        let channel = open_test_channel();
        let tmp_dir_path_thread = tmp_dir.path().to_owned();
        let handle = thread::spawn(move || {
            expect(&"Reading second level dir entry.", &channel);
            fs::remove_dir_all(tmp_dir_path_thread.join("socket-dir")).unwrap();
            send_continue(&channel);
            clear_expectations(channel);
        });

        match organize_socket_dir(&temp_dir_path) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e,
                format!(
                    "Error looking up socket information for socket {}/PAIR.zmq\nENOENT: No such file or directory.",
                    socket_dir.as_os_str().to_str().unwrap().to_owned()
                )
            ),
        };
        test_channels::report("END");
        assert_eq!(fs::read_dir(&tmp_dir).unwrap().count(), 0);
        tmp_dir.close().unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn test_unreadable_socket_dir() {
        use std::fs::set_permissions;
        use std::fs::Permissions;
        use std::os::unix::fs::PermissionsExt;
        use tempdir::TempDir;
        let tmp_dir = TempDir::new("test_sockets_dir4").unwrap();
        let empty_socket_dir = tmp_dir.path().join("empty-socket-dir");
        fs::create_dir(empty_socket_dir.clone()).unwrap();
        let no_permissions: Permissions = Permissions::from_mode(0o000);
        set_permissions(empty_socket_dir.clone(), no_permissions).unwrap();
        let temp_dir_path: String = tmp_dir.path().as_os_str().to_str().unwrap().to_owned();
        match organize_socket_dir(&temp_dir_path) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e,
                format!(
                    "Could not read directory {}\nPermission denied (os error 13)",
                    empty_socket_dir.as_os_str().to_str().unwrap().to_owned()
                )
            ),
        };
        let normal_permissions: Permissions = Permissions::from_mode(0o777);
        set_permissions(empty_socket_dir.clone(), normal_permissions).unwrap();
        let dir_listing: String = fs::read_dir(&tmp_dir)
            .unwrap()
            .map(|entry: Result<fs::DirEntry, std::io::Error>| {
                entry.unwrap().file_name().to_owned()
            })
            .map(|entry: std::ffi::OsString| entry.to_string_lossy().to_string())
            .collect();
        assert_eq!(dir_listing, "empty-socket-dir");
        tmp_dir.close().unwrap();
    }

    #[test]
    fn test_unwriteable_empty_socket_dir() {
        use std::fs::set_permissions;
        use std::fs::Permissions;
        use std::os::unix::fs::PermissionsExt;
        use tempdir::TempDir;
        let tmp_dir = TempDir::new("test_sockets_dir5").unwrap();
        let empty_socket_dir = tmp_dir.path().join("empty-socket-dir");
        fs::create_dir(empty_socket_dir.clone()).unwrap();
        // To trigger a write error when removing an empty socket dir
        // we set permissions on the parent as this is the easiest way to trigger
        // that condition.
        // https://github.com/rust-lang/rust/issues/92087
        let ro_permissions: Permissions = Permissions::from_mode(0o544);
        match set_permissions(tmp_dir.path(), ro_permissions) {
            Ok(_) => assert!(true),
            Err(_) => unreachable!(),
        };
        let temp_dir_path: String = tmp_dir.path().as_os_str().to_str().unwrap().to_owned();
        match organize_socket_dir(&temp_dir_path) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e,
                format!(
                    "Could not remove dir {}\nPermission denied (os error 13)",
                    empty_socket_dir.as_os_str().to_str().unwrap().to_owned()
                )
            ),
        };
        let normal_permissions: Permissions = Permissions::from_mode(0o777);
        set_permissions(tmp_dir.path(), normal_permissions).unwrap();
        assert_eq!(fs::read_dir(&tmp_dir).unwrap().count(), 1);
        tmp_dir.close().unwrap();
    }

    #[test]
    fn test_fail_on_dirty_socket_dir() {
        use tempdir::TempDir;
        let tmp_dir = TempDir::new("test_sockets_dir6").unwrap();
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
                unreachable!();
            }
        }
        assert!(contains);
        let temp_dir_path: String = tmp_dir.path().as_os_str().to_str().unwrap().to_owned();
        match organize_socket_dir(&temp_dir_path) {
            Ok(_) => unreachable!(),
            Err(error) => assert_eq!(
                error,
                format!(
                    "Unexpected files in socket dir \n{}/mess",
                    dirty_socket_dir.as_path().display()
                )
            ),
        }
        tmp_dir.close().unwrap();
    }
}
