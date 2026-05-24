use std::error::Error;
use std::thread::sleep;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};
use hyperliquid_stock_funding::{
    DEFAULT_API_URL, fetch_asset_snapshots, fetch_funding_history, fetch_stock_coins,
    funding_summary,
};
use reqwest::blocking::Client;

#[derive(Parser)]
#[command(
    name = "hl-stock-funding",
    about = "Calculate total and annualized funding for Hyperliquid stock perps."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(value_name = "COIN")]
    coins: Vec<String>,

    #[arg(short, long, default_value_t = 30)]
    days: u64,

    #[arg(long, default_value = DEFAULT_API_URL)]
    api_url: String,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Show current open interest for Hyperliquid stock perps.")]
    Oi {
        #[arg(value_name = "COIN")]
        coins: Vec<String>,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let client = Client::new();

    match cli.command {
        Some(Command::Oi { coins }) => run_oi(&client, &cli.api_url, coins),
        None => run_funding(&client, &cli.api_url, cli.coins, cli.days),
    }
}

fn run_funding(
    client: &Client,
    api_url: &str,
    coins: Vec<String>,
    days: u64,
) -> Result<(), Box<dyn Error>> {
    let coins = if coins.is_empty() {
        fetch_stock_coins(client, api_url)?
    } else {
        coins
    };

    let end_time = current_time_ms()?;
    let start_time = end_time.saturating_sub(days * 24 * 60 * 60 * 1000);
    let mut summaries = Vec::new();

    for coin in coins {
        let records = fetch_funding_history(client, api_url, &coin, start_time, end_time)?;
        summaries.push(funding_summary(&coin, &records, days as f64));
        sleep(Duration::from_millis(250));
    }

    summaries.sort_by(|a, b| b.annualized_rate.total_cmp(&a.annualized_rate));

    println!(
        "{:<18} {:>8} {:>14} {:>14}",
        "coin", "samples", "total", "annualized"
    );
    for summary in summaries {
        println!(
            "{:<18} {:>8} {:>13.4}% {:>13.2}%",
            summary.coin,
            summary.samples,
            summary.total_rate * 100.0,
            summary.annualized_rate * 100.0
        );
    }

    Ok(())
}

fn run_oi(client: &Client, api_url: &str, coins: Vec<String>) -> Result<(), Box<dyn Error>> {
    let coins = if coins.is_empty() {
        fetch_stock_coins(client, api_url)?
    } else {
        coins
    };
    let snapshots = fetch_asset_snapshots(client, api_url, &coins)?;

    println!(
        "{:<18} {:>14} {:>14} {:>14} {:>12} {:>12}",
        "coin", "oi", "oi_usd", "24h_volume", "funding", "premium"
    );
    for snapshot in snapshots {
        println!(
            "{:<18} {:>14.4} {:>14.0} {:>14.0} {:>11.4}% {:>11.4}%",
            snapshot.coin,
            snapshot.open_interest,
            snapshot.oi_usd(),
            snapshot.day_ntl_vlm.unwrap_or(0.0),
            snapshot.funding.unwrap_or(0.0) * 100.0,
            snapshot.premium.unwrap_or(0.0) * 100.0,
        );
    }

    Ok(())
}

fn current_time_ms() -> Result<u64, Box<dyn Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64)
}
