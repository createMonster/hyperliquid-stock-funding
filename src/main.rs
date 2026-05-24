use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use hyperliquid_stock_funding::{
    DEFAULT_API_URL, fetch_funding_history, fetch_stock_coins, funding_summary,
};
use reqwest::blocking::Client;

#[derive(Parser)]
#[command(
    name = "hl-stock-funding",
    about = "Calculate total and annualized funding for Hyperliquid stock perps."
)]
struct Cli {
    #[arg(value_name = "COIN")]
    coins: Vec<String>,

    #[arg(short, long, default_value_t = 30)]
    days: u64,

    #[arg(long, default_value = DEFAULT_API_URL)]
    api_url: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let client = Client::new();
    let coins = if cli.coins.is_empty() {
        fetch_stock_coins(&client, &cli.api_url)?
    } else {
        cli.coins
    };

    let end_time = current_time_ms()?;
    let start_time = end_time.saturating_sub(cli.days * 24 * 60 * 60 * 1000);
    let mut summaries = Vec::new();

    for coin in coins {
        let records = fetch_funding_history(&client, &cli.api_url, &coin, start_time, end_time)?;
        summaries.push(funding_summary(&coin, &records, cli.days as f64));
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

fn current_time_ms() -> Result<u64, Box<dyn Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64)
}
