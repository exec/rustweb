use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tracing::{error, info};

mod compression;
mod config;
mod config_reload;
mod logging;
mod metrics;
mod proxy;
mod security;
mod server;
mod ssl_cert_gen;

use config::Config;
use server::HttpServer;

#[derive(Parser)]
#[command(name = "rustweb")]
#[command(about = "A high-performance HTTP server written in Rust")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Args {
    #[arg(short, long, default_value = "/etc/rustweb/rustweb.toml")]
    config: String,

    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    #[arg(short, long, default_value = "8080")]
    port: u16,

    #[arg(short, long)]
    daemon: bool,

    #[arg(short = 't', long)]
    test_config: bool,

    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    init_logging(args.verbose)?;

    if args.test_config {
        let config = Config::load(&args.config)?;
        info!("Configuration file {} is valid", args.config);
        config.validate()?;
        println!("Configuration test successful");
        return Ok(());
    }

    let config = Config::load(&args.config).unwrap_or_else(|e| {
        error!("Failed to load configuration: {}", e);
        info!("Using default configuration");
        Config::default_with_host_port(&args.host, args.port)
    });

    if args.daemon {
        daemonize()?;
    }

    let server = HttpServer::new(Arc::new(config))?;
    server.run().await?;

    Ok(())
}

fn init_logging(verbose: bool) -> Result<()> {
    let env_filter = if verbose {
        "rustweb=debug,tower_http=debug"
    } else {
        "rustweb=info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .init();

    Ok(())
}

fn daemonize() -> Result<()> {
    #[cfg(unix)]
    {
        use std::process;
        match unsafe { libc::fork() } {
            -1 => return Err(anyhow::anyhow!("Failed to fork process")),
            0 => {
                if unsafe { libc::setsid() } == -1 {
                    return Err(anyhow::anyhow!("Failed to create new session"));
                }

                unsafe {
                    libc::close(0);
                    libc::close(1);
                    libc::close(2);
                }
            }
            _ => process::exit(0),
        }
    }

    #[cfg(not(unix))]
    {
        return Err(anyhow::anyhow!(
            "Daemon mode not supported on this platform"
        ));
    }

    Ok(())
}
