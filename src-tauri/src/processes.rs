use std::process::Command;

/// Terminate descendants before their parent, including shell job-control groups.
pub fn kill_tree(pid: u32) {
    #[cfg(unix)]
    {
        use nix::{sys::signal::{kill, killpg, Signal}, unistd::Pid};
        let mut descendants = vec![pid];
        if let Ok(output) = Command::new("ps").args(["-eo", "pid=,ppid=,sid=,pgid="]).output() {
            let pairs: Vec<(u32, u32, u32, u32)> = String::from_utf8_lossy(&output.stdout).lines().filter_map(|line| {
                let mut fields = line.split_whitespace();
                Some((fields.next()?.parse().ok()?, fields.next()?.parse().ok()?, fields.next()?.parse().ok()?, fields.next()?.parse().ok()?))
            }).collect();
            let mut index = 0;
            while index < descendants.len() {
                let parent = descendants[index];
                for &(child, parent_id, session_id, group_id) in &pairs {
                    if (parent_id == parent || session_id == pid || group_id == pid) && !descendants.contains(&child) { descendants.push(child); }
                }
                index += 1;
            }
        }
        for &child in descendants.iter().rev() {
            if let Ok(raw) = i32::try_from(child) {
                let _ = kill(Pid::from_raw(raw), Signal::SIGKILL);
            }
        }
        if let Ok(raw) = i32::try_from(pid) { let _ = killpg(Pid::from_raw(raw), Signal::SIGKILL); }
        #[cfg(target_os = "linux")]
        for child in descendants.into_iter().filter(|child| *child != pid) {
            let Ok(raw) = i32::try_from(child) else { continue; };
            let child = Pid::from_raw(raw);
            for _ in 0..50 {
                match nix::sys::wait::waitpid(child, Some(nix::sys::wait::WaitPidFlag::WNOHANG)) {
                    Ok(nix::sys::wait::WaitStatus::StillAlive) | Err(nix::errno::Errno::ECHILD) => {
                        if kill(child, None).is_err() { break; }
                        std::thread::sleep(std::time::Duration::from_millis(2));
                    }
                    _ => break,
                }
            }
        }
    }
    #[cfg(windows)]
    { let _ = Command::new("taskkill").args(["/PID", &pid.to_string(), "/T", "/F"]).status(); }
}

pub fn enable_reaping() {
    #[cfg(target_os = "linux")]
    if let Err(error) = nix::sys::prctl::set_child_subreaper(true) {
        eprintln!("Nicle: could not enable descendant reaping: {error}");
    }
}

pub fn configure_group(command: &mut Command) {
    enable_reaping();
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    #[test]
    fn completed_parent_does_not_leave_background_children() -> std::io::Result<()> {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "sleep 60 & echo $!"]).stdout(Stdio::piped());
        configure_group(&mut command);
        let mut child = command.spawn()?;
        let stdout = child.stdout.take().ok_or_else(|| std::io::Error::other("missing stdout"))?;
        let mut pid = String::new();
        BufReader::new(stdout).read_line(&mut pid)?;
        assert!(child.wait()?.success());
        kill_tree(child.id());
        let process = Command::new("ps").args(["-o", "stat=", "-p", pid.trim()]).output()?;
        assert!(process.stdout.is_empty(), "background child left behind: {}", String::from_utf8_lossy(&process.stdout));
        Ok(())
    }
    #[test]
    fn stopping_process_tree_terminates_the_running_child() -> std::io::Result<()> {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "sleep 60 & echo $!; wait"]).stdout(Stdio::piped());
        configure_group(&mut command);
        let mut child = command.spawn()?;
        let stdout = child.stdout.take().ok_or_else(|| std::io::Error::other("missing stdout"))?;
        let mut pid = String::new();
        BufReader::new(stdout).read_line(&mut pid)?;
        kill_tree(child.id());
        assert!(!child.wait()?.success());
        let process = Command::new("ps").args(["-o", "stat=", "-p", pid.trim()]).output()?;
        assert!(process.stdout.is_empty(), "descendant left behind: {}", String::from_utf8_lossy(&process.stdout));
        Ok(())
    }
}
