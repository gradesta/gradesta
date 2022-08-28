use nix::fcntl;
use std::fs;
use std::io::{Read, Write};
use std::path;
use sysinfo;
use sysinfo::{ProcessExt, SystemExt};

#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(target_os = "wasi")]
use std::os::wasi::io::{AsRawFd, RawFd};
use std::io::Seek;

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
                "Error opening lock file {:?}: {}",
                lockfile.as_os_str(),
                err.to_string()
            ))
        })?;
    fcntl::flock(f.as_raw_fd(), fcntl::FlockArg::LockExclusive).or_else(|err| {
        Err(anyhow!(
            "Error locking file {:?}: {}",
            lockfile.as_os_str(),
            err
        ))
    })?;
    if exists {
        let mut contents = String::new();
        f.read_to_string(&mut contents).or_else(|err| {
            Err(anyhow!(
                "Error reading lock file {:?}: {}",
                lockfile.as_os_str(),
                err
            ))
        })?;
        let old_pid: i32 = contents.parse::<i32>().or_else(|err| {
            Err(anyhow!(
                "Corrupted lock file {:?}: {}",
                lockfile.as_os_str(),
                err.to_string()
            ))
        })?;
        let mut system_info = sysinfo::System::new();
        system_info.refresh_process(old_pid);
        match system_info.process(old_pid) {
            Some(process) => {
                return Err(anyhow!(
                    "The sockets directory is already locked by pid {} {}",
                    process.pid(),
                    process.name()
                ))
            }
            None => (),
        };
    }
    f.seek(std::io::SeekFrom::Start(0))?;
    f.set_len(0)?;
    f.write(&format!("{}", std::process::id()).as_bytes())
        .or_else(|err| {
            Err(anyhow!(
                "Error writing to lock file {:?}: {}",
                lockfile.as_os_str(),
                err
            ))
        })?;
    fcntl::flock(f.as_raw_fd(), fcntl::FlockArg::Unlock).or_else(|err| {
        Err(anyhow!(
            "Error unlocking lock file {:?}: {}",
            lockfile.as_os_str(),
            err
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
        match lock_sockets_dir(&tmp_dir.path()) {
            Ok(_) => unreachable!(),
            Err(e) => assert!(e.to_string().starts_with("The sockets directory is already locked by pid ")),
        }
        let mut lockfile = fs::OpenOptions::new()
            .write(true)
            .open(lockfile_path)
            .unwrap();
        lockfile.write("not a pid".as_bytes()).unwrap();
        match lock_sockets_dir(&tmp_dir.path()) {
            Ok(_) => unreachable!(),
            Err(e) => assert!(e.to_string().starts_with("Corrupted lock file ")),
        }
    }
}
