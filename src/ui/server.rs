//! Embedded HTTP server lifecycle management for K0maru dashboard.

use std::path::PathBuf;

use crate::ui::routes::{create_router, AppState};

/// Runs the local dashboard embedded server bound to 127.0.0.1:{port}.
pub async fn run_server(
    vault_path: PathBuf,
    port: u16,
    open: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = AppState::new(vault_path)?;
    let app = create_router(state);

    let bind_addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    let url = format!("http://127.0.0.1:{}", port);
    println!("🚀 K0maru Dashboard active on {}", url);

    if open {
        open_browser(&url);
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn open_browser(url: &str) {
    println!("Opening {} in default browser...", url);
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", url])
        .spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    println!("\nShutting down K0maru Dashboard server...");
}
