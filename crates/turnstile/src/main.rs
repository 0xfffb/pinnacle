mod config;

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use clap::Parser;
use pingora::prelude::*;
use pingora::proxy::http_proxy_service;
use pingora::server::ShutdownWatch;
use pingora::services::background::BackgroundService;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::EnvFilter;

use config::Config;
use pinnacle_control::ControlServer;
use pinnacle_turnstile::Turnstile;

// ── Pingora adapter ─────────────────────────────────────────────────────────
// control crate 对 Pingora 零依赖；适配在这里完成。

struct ControlAdapter(Arc<ControlServer>);

#[async_trait]
impl BackgroundService for ControlAdapter {
    async fn start(&self, mut _shutdown: ShutdownWatch) {
        self.0.start().await;
    }
}

// ────────────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "pinnacled", about = "Pinnacle anti-bot gateway daemon")]
struct Cli {
    #[arg(long, default_value = "pinnacle.toml")]
    config: PathBuf,
}

fn main() {
    let cli = Cli::parse();
    init_log();

    let cfg = Config::load(&cli.config).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });

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
