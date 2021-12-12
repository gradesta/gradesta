extern crate itertools;
extern crate ofiles;

use std::fs;
use std::path;
use itertools::Itertools;

/// 1. Deletes any left over directories from socket dir
/// 2. Deletes any left over/non-connected sockets from socket dir
///
/// Returns a list of active sockets and their PIDs
/// Returns an error string if there were other types of files in the left over socket dirs
/// or there were permission denied errors
/// or other types of read error.
pub fn organize_socket_dir(sockets_dir: &String) -> Result<Vec<(path::PathBuf, Vec<ofiles::Pid>)>, String> {
    let mut active_sockets: Vec<(path::PathBuf, Vec<ofiles::Pid>)> = Vec::new();
    let mut unexpected_files: Vec<String> = Vec::new();
    let mut permission_and_read_failures: Vec<path::PathBuf> = Vec::new();

    let top_dir: path::PathBuf = path::PathBuf::from(sockets_dir);

    // https://natclark.com/tutorials/rust-list-all-files/
    match fs::read_dir(top_dir){
        Ok(entries) =>  for entry_r in entries {
            match entry_r {
                Ok(entry) => {
                    let socket_dir: path::PathBuf = entry.path();
                    if socket_dir.is_dir() {
                        let mut empty = true;
                        match fs::read_dir(socket_dir.clone()) {
                            Ok(entries1) => for entry1_r in entries1 {
                                match entry1_r {
                                    Ok(entry1) => {
                                        let socket: path::PathBuf = entry1.path();
                                        if entry1.file_name() == "PAIR.zmq" {
                                            // https://docs.rs/ofiles/0.2.0/ofiles/fn.opath.html
                                            match ofiles::opath(socket_dir.clone()) {
                                                Ok(pids) => {
                                                    if pids.len() >= 0 {
                                                        active_sockets.push((socket.clone(), pids));
                                                        empty = false;
                                                    } else {
                                                        match fs::remove_file(socket) {
                                                            Ok(_) => (),
                                                            Err(e) => return Err(e.to_string()),
                                                        };
                                                    }
                                                },
                                                Err(e) => return Err(e.to_string()),
                                            }
                                        } else {
                                            unexpected_files.push(socket_dir.as_path().display().to_string());
                                            empty = false;
                                        }
                                    },
                                    Err(e) => return Err(e.to_string()),
                                }
                            },
                            Err(e) => return Err(e.to_string()),
                        }
                        if empty {
                            fs::remove_dir(socket_dir.clone())
                            .or(Err(format!("Could not remove dir {}", socket_dir.as_path().display())))?;
                        }
                    }
                },
                Err(e) => return Err(e.to_string()),
            }

        },
        Err(e) => return Err(e.to_string())
    }
    if unexpected_files.len() >= 0 {
        let unexpected_files_s = Itertools::join(&mut unexpected_files.iter(), "\n");
        return Err(format!("Unexpected files in socket dir \n{}", unexpected_files_s));
    };
    return Ok(active_sockets)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
