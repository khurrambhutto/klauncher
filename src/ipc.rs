use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::{
        fs::{DirBuilderExt, FileTypeExt, OpenOptionsExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::PathBuf,
    sync::mpsc,
    time::Duration,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Command {
    Show {
        query: Option<String>,
        activation_token: Option<String>,
    },
    Toggle,
    Hide,
    Status,
    Quit,
}

pub struct Request {
    pub command: Command,
    pub reply: mpsc::SyncSender<Value>,
}

pub struct Server {
    pub requests: async_channel::Receiver<Request>,
    path: PathBuf,
    _lock: fs::File,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn socket_path() -> Result<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .context("XDG_RUNTIME_DIR is missing; run the launcher inside a desktop session")?;
    let dir = PathBuf::from(runtime).join("raycast-linux");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)?;
    let metadata = fs::symlink_metadata(&dir)?;
    if !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
        bail!("Launcher runtime directory must be a private directory");
    }
    Ok(dir.join("launcher.sock"))
}

fn forward(stream: &mut UnixStream, command: &Command) -> Result<Value> {
    stream.set_read_timeout(Some(Duration::from_secs(4)))?;
    stream.set_write_timeout(Some(Duration::from_secs(1)))?;
    serde_json::to_writer(&mut *stream, command)?;
    stream.write_all(b"\n")?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    Ok(serde_json::from_str(&line)?)
}

pub enum Instance {
    Primary(Server),
    Forwarded(Value),
}

pub fn start(command: &Command) -> Result<Instance> {
    let path = socket_path()?;
    match UnixStream::connect(&path) {
        Ok(mut stream) => return Ok(Instance::Forwarded(forward(&mut stream, command)?)),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
            ) => {}
        Err(error) => return Err(error.into()),
    }
    if !matches!(command, Command::Show { .. } | Command::Toggle) {
        bail!("The launcher is not running");
    }
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path.with_extension("lock"))?;
    lock.try_lock()
        .context("Another launcher is starting; retry activation in a moment")?;
    if let Ok(mut stream) = UnixStream::connect(&path) {
        return Ok(Instance::Forwarded(forward(&mut stream, command)?));
    }
    if let Ok(metadata) = fs::symlink_metadata(&path) {
        if !metadata.file_type().is_socket() {
            bail!("Refusing to replace a non-socket runtime entry");
        }
        fs::remove_file(&path)?;
    }
    let listener = match UnixListener::bind(&path) {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            let mut stream = UnixStream::connect(&path)?;
            return Ok(Instance::Forwarded(forward(&mut stream, command)?));
        }
        Err(error) => return Err(error.into()),
    };
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    let (sender, requests) = async_channel::bounded(32);
    std::thread::Builder::new()
        .name("launcher-control".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
                let mut line = String::new();
                let result = std::io::Read::take(&mut stream, 8192);
                if BufReader::new(result).read_line(&mut line).is_err() {
                    continue;
                }
                let response = match serde_json::from_str::<Command>(&line) {
                    Ok(command) => {
                        let (reply, receive) = mpsc::sync_channel(1);
                        if sender.try_send(Request { command, reply }).is_err() {
                            serde_json::json!({"error":"Launcher is busy"})
                        } else {
                            receive.recv_timeout(Duration::from_secs(3)).unwrap_or_else(
                                |_| serde_json::json!({"error":"Launcher did not respond"}),
                            )
                        }
                    }
                    Err(_) => serde_json::json!({"error":"Invalid launcher request"}),
                };
                let _ = serde_json::to_writer(&mut stream, &response);
                let _ = stream.write_all(b"\n");
            }
        })?;
    Ok(Instance::Primary(Server {
        requests,
        path,
        _lock: lock,
    }))
}
