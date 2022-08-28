use crate::ageing_cellar::organize_sockets_dir::*;
use anyhow::anyhow;
use nix::sys::signal::{kill, Signal};
use std::collections;
use std::io::{BufRead, Write};
use std::path;
use std::thread;
use std::time::Duration;
use sysinfo;
use sysinfo::{ProcessExt, SystemExt};

/// Deletes left over sockets.
/// If these sockets are in use and in terminal
///  asks user if we should send TERM/KILL signal to the user
///  users can also "ignore" the process that is using the socket and just try to delete it anyways
/// If not in terminal reports error when sockets are in use.
pub fn clean(sockets_dir: &path::Path) -> anyhow::Result<()> {
    use is_terminal::IsTerminal;
    if std::io::stdout().is_terminal() {
        interactive_clean_and_kill(sockets_dir, std::io::stdin().lock(), std::io::stdout())
    } else {
        // Just print error message about dangling sockets
        let dangling_sockets = organize_sockets_dir(sockets_dir, &collections::HashSet::new())?;
        if dangling_sockets.len() > 0 {
            Err(anyhow!("Could not start because the following gradesta service sockets are still in use: \n{}", display_dangling_sockets(dangling_sockets)?))
        } else {
            Ok(())
        }
    }
}

fn display_pid(pid: ofiles::Pid) -> anyhow::Result<String> {
    let mut system_info = sysinfo::System::new();
    let i32pid: i32 = <i32>::from(pid).try_into()?;
    system_info.refresh_process(i32pid);
    match system_info.process(i32pid) {
        Some(process) => {
            return Ok(format!("    {} - {}\n", i32pid, process.name()));
        }
        None => return Ok(format!("    {} - no longer running\n", i32pid)),
    };
}

/// Returns a string containing a nicely formatted listing of the dangling sockets
/// and the processes that are using them.
/// For example if there are two dangling sockets being used collectively by 3 programs,
/// the display will appear as follows
/// /path/to/socket.sock - currently used by
///    PID - name
///    343 - cargo-watch
///    1002 - gradesta-fileserve
/// /path/to/another/socket.sock
///    PID - name
///    343 - cargo-watch
///    3940 - gradesta-chess-demo
fn display_dangling_sockets(
    dangling_sockets: Vec<(path::PathBuf, Vec<ofiles::Pid>)>,
) -> anyhow::Result<String> {
    let mut display = String::new();
    for (socket, pids) in dangling_sockets {
        let socket_header: String = format!("{}\n    PID - name\n", socket.to_string_lossy());
        display.push_str(&socket_header);
        for pid in pids {
            display.push_str(&display_pid(pid)?);
        }
    }
    return Ok(display);
}

fn interactive_clean_and_kill<R, W>(
    sockets_dir: &path::Path,
    mut stdin: R,
    mut stdout: W,
) -> anyhow::Result<()>
where
    R: BufRead,
    W: Write,
{
    let mut ignored_pids: collections::HashSet<ofiles::Pid> = collections::HashSet::new();
    'main_loop: loop {
        let dangling_sockets = organize_sockets_dir(sockets_dir, &ignored_pids)?;
        for (socket, pids) in dangling_sockets {
            for pid in pids {
                // Prompt the user as to what to do with the pid
                writeln!(stdout, "The socket {} is currently in use by the following program\n    PID - name\n{}Would you like to [t term, k kill, i ignore, w wait 1 sec] this process?", socket.to_string_lossy(), display_pid(pid)?)?;
                // Actually do the killing/terminating/ignoring
                let mut s = String::new();
                stdin.read_line(&mut s)?;
                let i32pid = <i32>::from(pid);
                let nixpid = nix::unistd::Pid::from_raw(i32pid);
                match s.as_str() {
                    "t\n" => {
                        kill(nixpid, Signal::SIGTERM)?;
                        thread::sleep(Duration::from_secs(1));
                    }
                    "k\n" => {
                        kill(nixpid, Signal::SIGKILL)?;
                        thread::sleep(Duration::from_secs(1));
                    }
                    "w\n" => {
                        thread::sleep(Duration::from_secs(1));
                    }
                    "i\n" => {
                        ignored_pids.insert(pid);
                    }
                    _ => {
                        write!(stdout, "Unknown option.")?;
                    }
                };
                continue 'main_loop;
            }
        }
        break Ok(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_interactive_clean(input: &[u8], output: &mut Vec<u8>) -> (i32, String) {
        use tempdir::TempDir;
        let tmp_dir = TempDir::new("test_interactive_clean_sockets_dir").unwrap();
        let socket_dir_path = tmp_dir.into_path();
        std::fs::create_dir(socket_dir_path.join("socket-dir")).unwrap();
        let socket_dir_str: String = socket_dir_path.to_string_lossy().to_string();
        use test_binary::*;
        let test_bin_path = build_test_binary("test-socket-holder", "testbins")
            .expect("error building test binary");
        let socket_path = format!("{}/socket-dir/PAIR.zmq", &socket_dir_str);
        let test_bin_subproc = std::process::Command::new(test_bin_path)
            .arg(format!("ipc://{}", &socket_path))
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Error running test binary");
        let subproc_pid = test_bin_subproc.id();
        let stdout = test_bin_subproc.stdout.unwrap();
        let mut stdout_reader = std::io::BufReader::new(stdout);
        let mut s = String::new();
        let _ = stdout_reader.read_line(&mut s);
        assert_eq!(s, "Ready!\n");
        interactive_clean_and_kill(&socket_dir_path, &input[..], output).unwrap();
        let mut s1 = String::new();
        let _ = stdout_reader.read_line(&mut s1);
        assert_eq!(s1, "TERM\n");
        return (subproc_pid.try_into().unwrap(), socket_path);
    }

    #[test]
    fn test_term_than_kill() {
        let input = b"t\nk\n";
        let mut output = Vec::new();
        let (pid, socket_path) = test_interactive_clean(input, &mut output);
        assert_eq!(std::str::from_utf8(&output).unwrap(), format!("The socket {0} is currently in use by the following program\n    PID - name\n    {1} - test-socket-hol\nWould you like to [t term, k kill, i ignore, w wait 1 sec] this process?\nThe socket {0} is currently in use by the following program\n    PID - name\n    {1} - test-socket-hol\nWould you like to [t term, k kill, i ignore, w wait 1 sec] this process?\n", &socket_path, pid));
        match procfs::process::Process::new(pid) {
            Err(_) => assert!(true), //Process was killed.
            Ok(process) => assert!(!process.is_alive()),
        }
    }

    #[test]
    fn test_term_than_ignore() {
        let input = b"t\ni\n";
        let mut output = Vec::new();
        let (pid, socket_path) = test_interactive_clean(input, &mut output);
        assert_eq!(std::str::from_utf8(&output).unwrap(), format!("The socket {0} is currently in use by the following program\n    PID - name\n    {1} - test-socket-hol\nWould you like to [t term, k kill, i ignore, w wait 1 sec] this process?\nThe socket {0} is currently in use by the following program\n    PID - name\n    {1} - test-socket-hol\nWould you like to [t term, k kill, i ignore, w wait 1 sec] this process?\n", &socket_path, pid));
        match procfs::process::Process::new(pid) {
            Err(_) => assert!(false), //Process was not killed.
            Ok(process) => assert!(process.is_alive()),
        }
    }
}
