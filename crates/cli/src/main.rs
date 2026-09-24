mod config;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use pingora::prelude::*;
use pingora::proxy::http_proxy_service;
use pingora::server::ShutdownWatch;
use pingora::services::background::BackgroundService;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::EnvFilter;

use config::Config;
use pinnacle_control::ControlClient;
use pinnacle_control::ControlServer;
use pinnacle_turnstile::Turnstile;

// ── Pingora adapter ──────────────────────────────────────────────────────────

struct ControlAdapter(Arc<ControlServer>);

#[async_trait]
impl BackgroundService for ControlAdapter {
    async fn start(&self, mut _shutdown: ShutdownWatch) {
        self.0.start().await;
    }
}

// ────────────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "pinnacle", about = "Pinnacle anti-bot gateway")]
struct Cli {
    #[arg(long, default_value = "pinnacle.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Start the gateway daemon (foreground)
    Run,

    /// Show request and ban counters
    Stats,

    /// Manage bans
    Ban {
        #[command(subcommand)]
        sub: BanCmd,
    },

    /// Inspect a single IP
    Inspect {
        /// IP address to inspect
        ip: String,
    },
}

#[derive(Subcommand, Debug)]
enum BanCmd {
    /// List all banned IPs
    Ls,

    /// Ban an IP address
    Add {
        /// IP address to ban
        ip: String,

        /// Reason for the ban
        #[arg(short, long, default_value = "manual")]
        reason: String,
    },

    /// Unban an IP address
    Rm {
        /// IP address to unban
        ip: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let cfg = Config::load(&cli.config).unwrap_or_else(|e| {
        eprintln!("config: {e}");
        std::process::exit(2);
    });

    match cli.command {
        Command::Run => run_daemon(cfg),
        cmd => {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            if let Err(e) = rt.block_on(run_ctl(cfg, cmd)) {
                eprintln!("error: {e:#}");
                std::process::exit(1);
            }
        }
    }
}

// ── Daemon ───────────────────────────────────────────────────────────────────

fn run_daemon(cfg: Config) {
    init_log();

    let upstream = cfg.upstream_peer().unwrap_or_else(|e| {
        eprintln!("invalid upstream: {e}");
        std::process::exit(2);
    });
    if let Err(e) = cfg.validate_listen() {
        eprintln!("invalid listen: {e}");
        std::process::exit(2);
    }

    let upstream_addr = format!("{}:{}", upstream.0, upstream.1);
    print_banner(&cfg.listen, &upstream_addr, &cfg.control_socket);

    let turnstile = Turnstile::new();
    let store = turnstile.state().store.clone();

    let mut server = Server::new(Some(Opt::default())).unwrap();
    server.bootstrap();

    let control = Arc::new(ControlServer::new(store, &cfg.control_socket));
    server.add_service(background_service("control-api", ControlAdapter(control)));

    let mut proxy = http_proxy_service(
        &server.configuration,
        pinnacle_gateway::Gateway::new(upstream, turnstile),
    );
    proxy.add_tcp(&cfg.listen);
    server.add_service(proxy);

    server.run_forever();
}

// ── Control CLI ──────────────────────────────────────────────────────────────

async fn run_ctl(cfg: Config, cmd: Command) -> anyhow::Result<()> {
    let client = ControlClient::new(&cfg.control_socket);

    match cmd {
        Command::Run => unreachable!(),

        Command::Stats => {
            let s = client.stats().await.context("stats")?;
            println!("requests : {}", s.requests_total);
            println!("banned   : {}", s.banned_total);
        }

        Command::Ban { sub: BanCmd::Ls } => {
            let res = client.bans().await.context("ban ls")?;
            if res.bans.is_empty() {
                println!("(no bans)");
            } else {
                for b in res.bans {
                    println!("{:40}  {}", b.ip, b.reason);
                }
            }
        }

        Command::Ban { sub: BanCmd::Add { ip, reason } } => {
            client.ban(&ip, &reason).await.context("ban add")?;
            println!("banned {ip}");
        }

        Command::Ban { sub: BanCmd::Rm { ip } } => {
            client.unban(&ip).await.context("ban rm")?;
            println!("unbanned {ip}");
        }

        Command::Inspect { ip } => {
            let r = client.inspect(&ip).await.context("inspect")?;
            println!("ip           : {}", r.ip);
            println!("banned       : {}", r.banned);
            if let Some(reason) = r.ban_reason {
                println!("ban reason   : {reason}");
            }
            println!("pass valid   : {}", r.pass_valid);
            println!("requests     : {}", r.request_count);
        }
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

const BANNER: &str = r#"
    ____  _                          __
   / __ \(_)___  ____  ____ ________/ /__
  / /_/ / / __ \/ __ \/ __ `/ ___/ / _ \
 / ____/ / / / / / / / /_/ / /__/ /  __/
/_/   /_/_/ /_/_/ /_/\__,_/\___/_/\___/
"#;

fn print_banner(listen: &str, upstream: &str, socket: &str) {
    info!(
        "\n{}\n  listen   {}\n  upstream {}\n  control  {}\n",
        BANNER.trim_start_matches('\n'),
        listen,
        upstream,
        socket,
    );
}

fn init_log() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
