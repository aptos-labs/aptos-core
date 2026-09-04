// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Solver processes that die together.
//!
//! Boogie runs the SMT solver as a child process. Killing Boogie alone on a
//! deadline re-parents that solver to init, where it keeps working at full CPU
//! on a query nobody waits for. [`ProcessGroupChild`] therefore spawns Boogie
//! as the leader of its own process group and kills the group whenever the
//! child is dropped before it has been waited for -- on a deadline, a lost
//! seed race, or a cancelled task. A leader that exited on its own is left
//! alone: Boogie ends its solver itself, and the group id may already have
//! been reused.
//!
//! Non-Unix targets have no process groups; there the child is only killed on
//! drop, as before.

use log::debug;
use std::process::{Output, Stdio};
use tokio::process::{Child, Command};

pub(crate) struct ProcessGroupChild {
    child: Option<Child>,
    #[cfg(unix)]
    group: Option<nix::unistd::Pid>,
}

impl ProcessGroupChild {
    /// Spawn `command` as the leader of a new process group, with no stdin.
    pub(crate) fn spawn(command: &mut Command) -> std::io::Result<Self> {
        command.stdin(Stdio::null()).kill_on_drop(true);
        #[cfg(unix)]
        command.process_group(0);
        let child = command.spawn()?;
        #[cfg(unix)]
        let group = child.id().map(|id| nix::unistd::Pid::from_raw(id as i32));
        Ok(Self {
            child: Some(child),
            #[cfg(unix)]
            group,
        })
    }

    /// Wait for the leader and collect its piped output. Dropping the future
    /// before it completes kills the group.
    pub(crate) async fn wait_with_output(mut self) -> std::io::Result<Output> {
        let child = self
            .child
            .take()
            .expect("child is present until waited for");
        let output = child.wait_with_output().await?;
        self.disarm();
        Ok(output)
    }

    /// Wait for the leader. Dropping the future before it completes kills the
    /// group.
    pub(crate) async fn wait(mut self) -> std::io::Result<std::process::ExitStatus> {
        let child = self
            .child
            .as_mut()
            .expect("child is present until waited for");
        let status = child.wait().await?;
        self.disarm();
        Ok(status)
    }

    fn disarm(&mut self) {
        #[cfg(unix)]
        {
            self.group = None;
        }
    }
}

impl Drop for ProcessGroupChild {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(group) = self.group.take() {
            debug!("killing solver process group {}", group);
            let _ = nix::sys::signal::killpg(group, nix::sys::signal::Signal::SIGKILL);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::ProcessGroupChild;
    use nix::{
        sys::signal::{kill, Signal},
        unistd::Pid,
    };
    use std::{process::Stdio, time::Duration};
    use tokio::{io::AsyncReadExt, process::Command};

    fn alive(pid: Pid) -> bool {
        kill(pid, None).is_ok()
    }

    /// A shell whose background grandchild's pid is its first output line.
    async fn spawn_tree() -> (ProcessGroupChild, Pid) {
        let mut command = Command::new("sh");
        command
            .args(["-c", "sleep 600 >/dev/null 2>&1 & echo $!; wait"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = ProcessGroupChild::spawn(&mut command).expect("spawn sh");
        let mut stdout = child
            .child
            .as_mut()
            .expect("child")
            .stdout
            .take()
            .expect("piped stdout");
        let mut line = String::new();
        let mut byte = [0u8; 1];
        while stdout.read_exact(&mut byte).await.is_ok() && byte[0] != b'\n' {
            line.push(byte[0] as char);
        }
        let grandchild = Pid::from_raw(line.trim().parse().expect("grandchild pid"));
        assert!(alive(grandchild));
        (child, grandchild)
    }

    async fn wait_until_gone(pid: Pid) -> bool {
        for _ in 0..50 {
            if !alive(pid) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        false
    }

    #[tokio::test]
    async fn dropping_before_wait_kills_the_grandchild() {
        let (child, grandchild) = spawn_tree().await;
        drop(child);
        assert!(
            wait_until_gone(grandchild).await,
            "grandchild {} survived",
            grandchild
        );
    }

    #[tokio::test]
    async fn abandoned_wait_kills_the_grandchild() {
        let (child, grandchild) = spawn_tree().await;
        let abandoned =
            tokio::time::timeout(Duration::from_millis(200), child.wait_with_output()).await;
        assert!(abandoned.is_err(), "the leader should still be waiting");
        assert!(
            wait_until_gone(grandchild).await,
            "grandchild {} survived",
            grandchild
        );
    }

    #[tokio::test]
    async fn completed_wait_leaves_the_group_alone() {
        let mut command = Command::new("sh");
        command
            .args(["-c", "sleep 600 >/dev/null 2>&1 & echo $!"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let child = ProcessGroupChild::spawn(&mut command).expect("spawn sh");
        let output = child.wait_with_output().await.expect("leader exits");
        assert!(output.status.success());
        let grandchild = Pid::from_raw(
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse()
                .expect("grandchild pid"),
        );
        assert!(alive(grandchild));
        let _ = kill(grandchild, Signal::SIGKILL);
    }
}
