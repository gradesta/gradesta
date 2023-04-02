use nix::fcntl;
use std::fs;
use std::io::{Read, Write};
use std::path;
use sysinfo;
use sysinfo::{ProcessExt, SystemExt};

use std::io::Seek;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(target_os = "wasi")]
use std::os::wasi::io::{AsRawFd, RawFd};

use super::localizer::*;

use anyhow::anyhow;

/// Locks the socket dir by writing the current PID into
/// PID.lock
///
/// If the socket dir is currently locked by a running
/// process, returns a string complaining about the problem.
/// If any unexpected problems occur, also returns a string
/// complaining about them.
pub fn lock_sockets_dir(dir: &path::Path) -> anyhow::Result<()> {
    let lockfile = dir.join("PID.lock");
    let exists = lockfile.exists();
    if !exists {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(&lockfile)
        .or_else(|err| {
            Err(anyhow!(
                "{}",
                l3(
                    "lock-sockets-dir-open-lock-error",
                    /* "Error opening lock file {:?}: {}" */
                    "err_code",
                    "GR9",
                    "file",
                    lockfile.as_os_str().to_string_lossy().to_string(),
                    "error",
                    err.to_string()
                )
            ))
        })?;
    fcntl::flock(f.as_raw_fd(), fcntl::FlockArg::LockExclusive).or_else(|err| {
        Err(anyhow!(
            "{}",
            l3(
                "lock-sockets-dir-lock-error",
                /* "Error locking lock file {:?}: {}" */
                "err_code",
                "GR10",
                "file",
                lockfile.as_os_str().to_string_lossy().to_string(),
                "error",
                err.to_string()
            )
        ))
    })?;
    if exists {
        let mut contents = String::new();
        f.read_to_string(&mut contents).or_else(|err| {
            Err(anyhow!(
                "{}",
                l3(
                    "lock-sockets-dir-read-error",
                    /* "Error reading lock file {:?}: {}" */
                    "err_code",
                    "GR11",
                    "file",
                    lockfile.as_os_str().to_string_lossy().to_string(),
                    "error",
                    err.to_string()
                )
            ))
        })?;
        let old_pid: i32 = contents.parse::<i32>().or_else(|err| {
            Err(anyhow!(
                "{}",
                l3(
                    "lock-sockets-dir-parse-error",
                    /* "Error parsing lock file {:?}: {}" */
                    "err_code",
                    "GR12",
                    "file",
                    lockfile.as_os_str().to_string_lossy().to_string(),
                    "error",
                    err.to_string()
                )
            ))
        })?;
        let mut system_info = sysinfo::System::new();
        system_info.refresh_process(old_pid);
        match system_info.process(old_pid) {
            Some(process) => {
                return Err(anyhow!(
                    "{}",
                    l3(
                        "lock-sockets-dir-already-locked",
                        /* "The sockets directory is already locked by pid {} {}" */
                        "err_code",
                        "GR13",
                        "pid",
                        old_pid.to_string(),
                        "process_name",
                        process.name().to_string()
                    )
                ));
            }
            None => (),
        };
    }
    f.seek(std::io::SeekFrom::Start(0))?;
    f.set_len(0)?;
    f.write(&format!("{}", std::process::id()).as_bytes())
        .or_else(|err| {
            Err(anyhow!(
                "{}",
                l3(
                    "lock-sockets-dir-write-error",
                    /* "Error writing lock file {:?}: {}" */
                    "err_code",
                    "GR14",
                    "file",
                    lockfile.as_os_str().to_string_lossy().to_string(),
                    "error",
                    err.to_string()
                )
            ))
        })?;
    fcntl::flock(f.as_raw_fd(), fcntl::FlockArg::Unlock).or_else(|err| {
        Err(anyhow!(
            "{}",
            l3(
                "lock-sockets-dir-unlock-error",
                /* "Error unlocking lock file {:?}: {}" */
                "err_code",
                "GR15",
                "file",
                lockfile.as_os_str().to_string_lossy().to_string(),
                "error",
                err.to_string()
            )
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate tempdir;
    use std::fs;

    #[test]
    fn test_lock_sockets_dir() {
        use tempdir::TempDir;
        let tmp_dir = TempDir::new("test_lock_sockets_dir1").unwrap();
        lock_sockets_dir(&tmp_dir.path()).unwrap();
        let lockfile_path = tmp_dir.path().join("PID.lock");
        assert!(lockfile_path.exists());
        let mut lockfile = fs::File::open(&lockfile_path).unwrap();
        let mut contents = String::new();
        lockfile.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, format!("{}", std::process::id()));
        drop(lockfile);
        let start_of_error = "GR13: The sockets directory is already locked by pid ";
        match lock_sockets_dir(&tmp_dir.path()) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                &remove_unicode_direction_chars(&e.to_string())[0..start_of_error.len()],
                start_of_error
            ),
        }
        let mut lockfile = fs::OpenOptions::new()
            .write(true)
            .open(lockfile_path)
            .unwrap();
        lockfile.write("not a pid".as_bytes()).unwrap();
        let start_of_error = "GR12: Error parsing lock file";
        match lock_sockets_dir(&tmp_dir.path()) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                &remove_unicode_direction_chars(&e.to_string())[0..start_of_error.len()],
                start_of_error
            ),
        }
    }
}
