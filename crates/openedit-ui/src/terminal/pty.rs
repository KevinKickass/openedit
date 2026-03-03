//! PTY session management using `portable-pty`.

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

/// A running PTY session (shell process + master PTY handle).
pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    /// Receiver for data from background reader thread.
    output_rx: mpsc::Receiver<Vec<u8>>,
    /// Flag to signal the reader thread to stop.
    alive: Arc<AtomicBool>,
    /// Background reader thread handle.
    _reader_thread: Option<std::thread::JoinHandle<()>>,
}

impl PtySession {
    /// Spawn a new shell session with the given terminal size.
    pub fn spawn_shell(cols: u16, rows: u16) -> anyhow::Result<Self> {
        let pty_system = native_pty_system();

        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // Determine the shell to use
        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(windows) {
                "cmd.exe".to_string()
            } else {
                "/bin/bash".to_string()
            }
        });

        let mut cmd = CommandBuilder::new(&shell);
        // Set interactive flags
        if shell.contains("bash") {
            cmd.arg("--login");
        } else if shell.contains("zsh") {
            cmd.arg("-l");
        }

        // Set TERM environment variable
        cmd.env("TERM", "xterm-256color");

        let child = pair.slave.spawn_command(cmd)?;

        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;

        // Spawn background reader thread for non-blocking output
        let (tx, rx) = mpsc::channel();
        let alive = Arc::new(AtomicBool::new(true));
        let alive_clone = alive.clone();

        let reader_thread = std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while alive_clone.load(Ordering::Relaxed) {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            master: pair.master,
            writer,
            child,
            output_rx: rx,
            alive,
            _reader_thread: Some(reader_thread),
        })
    }

    /// Try to read output from the PTY (non-blocking via channel).
    pub fn read_output(&mut self) -> anyhow::Result<Vec<u8>> {
        let mut output = Vec::new();
        // Drain all available data from the channel without blocking
        while let Ok(data) = self.output_rx.try_recv() {
            output.extend_from_slice(&data);
        }
        // Check if child has exited and no more data
        if output.is_empty() {
            match self.child.try_wait() {
                Ok(Some(_)) => return Err(anyhow::anyhow!("Process exited")),
                Ok(None) => {} // still running, just no data yet
                Err(e) => return Err(e.into()),
            }
        }
        Ok(output)
    }

    /// Write input to the PTY.
    pub fn write_input(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }

    /// Resize the PTY.
    pub fn resize(&mut self, cols: u16, rows: u16) -> anyhow::Result<()> {
        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    /// Kill the shell process.
    pub fn kill(&mut self) {
        self.alive.store(false, Ordering::Relaxed);
        let _ = self.child.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_session_types() {
        // Just verify the types compile correctly
        // Actually spawning a shell in CI may not work
        let _size = PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        };
    }
}
