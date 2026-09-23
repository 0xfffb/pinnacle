mod config;
mod dashboard;
mod mock;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use pingora::prelude::*;
use pingora::proxy::http_proxy_service;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::EnvFilter;

use config::Config;
use pinnacle_gateway::Gateway;

#[derive(Parser, Debug)]
#[command(name = "pinnacle", about = "Anti-bot gateway")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Path to TOML config
    #[arg(long, default_value = "pinnacle.toml", global = true)]
    config: PathBuf,

    /// Override listen address (host:port)
    #[arg(long, global = true)]
    listen: Option<String>,

    /// Override upstream address (host:port)
    #[arg(long, global = true)]
    upstream: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the gateway
    Serve,
    /// Live gateway dashboard (demo metrics)
    Top,
    /// Snapshot counters
    Stats,
    /// Process / listen / upstream
    Status,
    /// Probe gateway and upstream
    Health,
    /// List banned IPs
    Bans,
    /// Challenge / verify counters
    Verify,
    /// Recent challenge / ban / proxy events
    Logs,
    /// Per-IP challenge and pass state
    Inspect {
        ip: String,
    },
    /// Ban an IP (mock)
    Ban {
        #[arg(long)]
        ip: String,
        #[arg(long, default_value = "manual")]
        reason: String,
    },
    /// Remove an IP from the ban list (mock)
    Unban {
        #[arg(long)]
        ip: String,
    },
}

impl Cli {
    fn run(self) {
        let Cli {
            command,
            config,
            listen,
            upstream,
        } = self;
        let result = match command.unwrap_or(Command::Serve) {
            Command::Serve => {
                serve(config, listen, upstream);
                return;
            }
            Command::Top => dashboard::run(),
            Command::Stats => mock::stats(),
            Command::Status => mock::status(),
            Command::Health => mock::health(),
            Command::Bans => mock::bans(),
            Command::Verify => mock::verify(),
            Command::Logs => mock::logs(),
            Command::Inspect { ip } => mock::inspect(&ip),
            Command::Ban { ip, reason } => mock::ban(&ip, &reason),
            Command::Unban { ip } => mock::unban(&ip),
        };
        if let Err(e) = result {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

fn serve(config: PathBuf, listen: Option<String>, upstream: Option<String>) {
        init_log();

        let mut cfg = Config::load(&config).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2);
        });

        if let Some(listen) = listen {
            cfg.listen = listen;
        }
        if let Some(upstream) = upstream {
            cfg.upstream = upstream;
        }

        let upstream = cfg.upstream_peer().unwrap_or_else(|e| {
            eprintln!("invalid upstream: {e}");
            std::process::exit(2);
        });
        if let Err(e) = cfg.validate_listen() {
            eprintln!("invalid listen: {e}");
            std::process::exit(2);
        }

        let upstream_addr = format!("{}:{}", upstream.0, upstream.1);
        print_banner(&cfg.listen, &upstream_addr);

        let mut server = Server::new(Some(Opt::default())).unwrap();
        server.bootstrap();

        let mut proxy = http_proxy_service(
            &server.configuration,
            Gateway::new(upstream, pinnacle_turnstile::Turnstile::new()),
        );
        proxy.add_tcp(&cfg.listen);

        server.add_service(proxy);
        server.run_forever();
}

const BANNER: &str = r#"
    ____  _                          __
   / __ \(_)___  ____  ____ ________/ /__
  / /_/ / / __ \/ __ \/ __ `/ ___/ / _ \
 / ____/ / / / / / / / /_/ / /__/ /  __/
/_/   /_/_/ /_/_/ /_/\__,_/\___/_/\___/
"#;

fn print_banner(listen: &str, upstream: &str) {
    info!(
        "\n{}\n  listen    {}\n  upstream  {}\n",
        BANNER.trim_start_matches('\n'),
        listen,
        upstream
    );
}

fn init_log() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn main() {
    Cli::parse().run();
}
