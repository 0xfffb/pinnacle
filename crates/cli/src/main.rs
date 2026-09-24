mod config;

use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};

use config::Config;
use pinnacle_control::ControlClient;

#[derive(Parser, Debug)]
#[command(name = "pinnacle", about = "Pinnacle control CLI")]
struct Cli {
    #[arg(long, default_value = "pinnacle.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
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
    let cfg = Config::load(&cli.config).unwrap_or_else(|_| Config::default());

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    if let Err(e) = rt.block_on(run(cfg, cli.command)) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

async fn run(cfg: Config, cmd: Command) -> anyhow::Result<()> {
    let client = ControlClient::new(&cfg.control_socket);

    match cmd {
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
