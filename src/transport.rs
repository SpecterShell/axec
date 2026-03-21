use std::path::Path;

use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::Result;

pub trait AsyncIo: AsyncRead + AsyncWrite + Unpin + Send {}

impl<T> AsyncIo for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

pub type Connection = Box<dyn AsyncIo>;

pub struct Listener {
    inner: ListenerInner,
}

impl Listener {
    pub fn bind(path: &Path) -> Result<Self> {
        Ok(Self {
            inner: ListenerInner::bind(path)?,
        })
    }

    pub async fn accept(&mut self) -> Result<Connection> {
        self.inner.accept().await
    }
}

pub async fn connect(path: &Path) -> Result<Connection> {
    connect_inner(path).await
}

#[cfg(unix)]
type ListenerInner = UnixListenerImpl;

#[cfg(unix)]
struct UnixListenerImpl {
    listener: tokio::net::UnixListener,
}

#[cfg(unix)]
impl UnixListenerImpl {
    fn bind(path: &Path) -> Result<Self> {
        Ok(Self {
            listener: tokio::net::UnixListener::bind(path)?,
        })
    }

    async fn accept(&mut self) -> Result<Connection> {
        let (stream, _) = self.listener.accept().await?;
        Ok(Box::new(stream))
    }
}

#[cfg(unix)]
async fn connect_inner(path: &Path) -> Result<Connection> {
    Ok(Box::new(tokio::net::UnixStream::connect(path).await?))
}

#[cfg(windows)]
type ListenerInner = WindowsListenerImpl;

#[cfg(windows)]
struct WindowsListenerImpl {
    name: std::ffi::OsString,
    pending: tokio::net::windows::named_pipe::NamedPipeServer,
}

#[cfg(windows)]
impl WindowsListenerImpl {
    fn bind(path: &Path) -> Result<Self> {
        let name = path.as_os_str().to_owned();
        Ok(Self {
            pending: create_pipe_instance(&name, true)?,
            name,
        })
    }

    async fn accept(&mut self) -> Result<Connection> {
        let next = create_pipe_instance(&self.name, false)?;
        let connected = std::mem::replace(&mut self.pending, next);

        match connected.connect().await {
            Ok(()) => Ok(Box::new(connected)),
            Err(err)
                if err.raw_os_error()
                    == Some(windows_sys::Win32::Foundation::ERROR_PIPE_CONNECTED as i32) =>
            {
                Ok(Box::new(connected))
            }
            Err(err) => Err(err.into()),
        }
    }
}

#[cfg(windows)]
fn create_pipe_instance(
    name: &std::ffi::OsString,
    first_pipe_instance: bool,
) -> Result<tokio::net::windows::named_pipe::NamedPipeServer> {
    let mut options = tokio::net::windows::named_pipe::ServerOptions::new();
    options
        .first_pipe_instance(first_pipe_instance)
        .reject_remote_clients(true);
    Ok(options.create(name)?)
}

#[cfg(windows)]
async fn connect_inner(path: &Path) -> Result<Connection> {
    Ok(Box::new(
        tokio::net::windows::named_pipe::ClientOptions::new().open(path.as_os_str())?,
    ))
}
