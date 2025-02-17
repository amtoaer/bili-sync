use std::io;

use tokio::signal;

#[cfg(target_family = "windows")]
pub async fn terminate() -> io::Result<()> {
    signal::ctrl_c().await
}

/// ctrl + c 发送的是 SIGINT 信号，docker stop 发送的是 SIGTERM 信号，都需要处理
#[cfg(target_family = "unix")]
pub async fn terminate() -> io::Result<()> {
    use tokio::select;

    let mut term = signal::unix::signal(signal::unix::SignalKind::terminate())?;
    let mut int = signal::unix::signal(signal::unix::SignalKind::interrupt())?;
    select! {
        _ = term.recv() => Ok(()),
        _ = int.recv() => Ok(()),
    }
}
