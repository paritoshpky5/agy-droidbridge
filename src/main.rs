mod api;
mod cli;
mod config;
mod conversation;
mod models;
mod security;

use std::future::IntoFuture;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if let [flag, file] = args.as_slice() {
        if flag == "--check-models" {
            return match models::Catalog::load(Some(std::path::Path::new(file))) {
                Ok(catalog) => {
                    println!("Model list is valid ({} models).", catalog.0.len());
                    std::process::ExitCode::SUCCESS
                }
                Err(_) => {
                    eprintln!(
                        "Invalid model list. Expected an agy model ID and display label on each line."
                    );
                    std::process::ExitCode::FAILURE
                }
            };
        }
        if flag == "--prepare-workspace" {
            return match cli::prepare_workspace(std::path::Path::new(file)) {
                Ok(_) => std::process::ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("Could not prepare the DroidBridge workspace: {error}");
                    std::process::ExitCode::FAILURE
                }
            };
        }
    }
    if !args.is_empty() {
        eprintln!("Usage: agy-droidbridge [--check-models FILE | --prepare-workspace DIR]");
        return std::process::ExitCode::FAILURE;
    }
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    if let Err(error) = serve().await {
        tracing::error!("{error}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::read()?;
    let catalog = models::Catalog::load(config.models_file.as_deref())
        .map_err(|_| "unable to load AGY_MODELS_FILE")?;
    let cli = cli::Executor::new(
        config.bin,
        config.timeout,
        config.pending,
        config.output_bytes,
        config.workspace,
    )
    .map_err(|error| format!("unable to initialize private CLI workspace: {error}"))?;
    let listener = TcpListener::bind(config.address)
        .await
        .map_err(|error| format!("unable to listen on {}: {error}", config.address))?;
    let router = api::router(
        cli,
        catalog,
        config.key,
        config.body_bytes,
        config.prompt_bytes,
    );
    let (stop, stopping) = oneshot::channel();
    let server = axum::serve(listener, router).with_graceful_shutdown(async {
        let _ = stopping.await;
    });
    let mut server = std::pin::pin!(server.into_future());
    tracing::info!(address = %config.address, "proxy listening");

    tokio::select! {
        result = &mut server => result?,
        () = shutdown_signal() => {
            let _ = stop.send(());
            match tokio::time::timeout(Duration::from_secs(5), &mut server).await {
                Ok(result) => result?,
                Err(_) => tracing::warn!("graceful shutdown deadline reached"),
            }
        }
    }
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    if let Ok(mut termination) =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
    {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = termination.recv() => {},
        }
        return;
    }
    let _ = tokio::signal::ctrl_c().await;
}
