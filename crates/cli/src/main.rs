mod config;

use std::path::PathBuf;

use clap::Parser;
use pingora::prelude::*;
use pingora::proxy::http_proxy_service;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::EnvFilter;

use config::Config;
use pinnacle_gateway::Gateway;
use pinnacle_turnstile::Turnstile;

#[derive(Parser, Debug)]
#[command(name = "pinnacle", about = "Anti-bot gateway")]
struct Cli {
    /// Path to TOML config
    #[arg(long, default_value = "pinnacle.toml")]
    config: PathBuf,

    /// Override listen address (host:port)
    #[arg(long)]
    listen: Option<String>,

    /// Override upstream address (host:port)
    #[arg(long)]
    upstream: Option<String>,
}

impl Cli {
    fn run(self) {
        init_log();

        let mut cfg = Config::load(&self.config).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2);
        });

        if let Some(listen) = self.listen {
            cfg.listen = listen;
        }
        if let Some(upstream) = self.upstream {
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

        let turnstile = Turnstile::new(cfg.policy());

        let mut server = Server::new(Some(Opt::default())).unwrap();
        server.bootstrap();

        let mut proxy =
            http_proxy_service(&server.configuration, Gateway::new(upstream, turnstile));
        proxy.add_tcp(&cfg.listen);

        server.add_service(proxy);
        server.run_forever();
    }
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
