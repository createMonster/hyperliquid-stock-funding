use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::json;

pub const DEFAULT_API_URL: &str = "https://api.hyperliquid.xyz/info";

#[derive(Debug, Clone, Deserialize)]
pub struct FundingRecord {
    pub coin: String,
    pub time: u64,
    #[serde(rename = "fundingRate")]
    pub funding_rate: String,
}

impl FundingRecord {
    pub fn new(coin: impl Into<String>, time: u64, funding_rate: impl Into<String>) -> Self {
        Self {
            coin: coin.into(),
            time,
            funding_rate: funding_rate.into(),
        }
    }

    pub fn rate(&self) -> f64 {
        self.funding_rate.parse().unwrap_or(0.0)
    }
}

#[derive(Debug, Clone)]
pub struct FundingSummary {
    pub coin: String,
    pub samples: usize,
    pub total_rate: f64,
    pub annualized_rate: f64,
}

pub fn stock_coins(mut categories: Vec<(String, String)>) -> Vec<String> {
    categories.retain(|(_, category)| category.eq_ignore_ascii_case("stocks"));
    categories.sort_by(|a, b| a.0.cmp(&b.0));
    categories.into_iter().map(|(coin, _)| coin).collect()
}

pub fn funding_summary(coin: &str, records: &[FundingRecord], days: f64) -> FundingSummary {
    let total_rate = records.iter().map(FundingRecord::rate).sum();

    FundingSummary {
        coin: coin.to_string(),
        samples: records.len(),
        total_rate,
        annualized_rate: annualized_rate(total_rate, days),
    }
}

pub fn annualized_rate(total_rate: f64, days: f64) -> f64 {
    if days <= 0.0 {
        0.0
    } else {
        total_rate * 365.0 / days
    }
}

pub fn next_page_start(records: &[FundingRecord]) -> Option<u64> {
    records.last().map(|record| record.time + 1)
}

pub fn fetch_stock_coins(client: &Client, api_url: &str) -> Result<Vec<String>, reqwest::Error> {
    let categories: Vec<(String, String)> = client
        .post(api_url)
        .json(&json!({ "type": "perpCategories" }))
        .send()?
        .error_for_status()?
        .json()?;

    Ok(stock_coins(categories))
}

pub fn fetch_funding_history(
    client: &Client,
    api_url: &str,
    coin: &str,
    start_time: u64,
    end_time: u64,
) -> Result<Vec<FundingRecord>, reqwest::Error> {
    let mut start_time = start_time;
    let mut records = Vec::new();

    while start_time <= end_time {
        let page: Vec<FundingRecord> = client
            .post(api_url)
            .json(&json!({
                "type": "fundingHistory",
                "coin": coin,
                "startTime": start_time,
                "endTime": end_time
            }))
            .send()?
            .error_for_status()?
            .json()?;

        if page.is_empty() {
            break;
        }

        let next_start = next_page_start(&page);
        let is_last_page = page.len() < 500;
        records.extend(page);

        match (is_last_page, next_start) {
            (true, _) | (_, None) => break,
            (false, Some(next_start)) => start_time = next_start,
        }
    }

    Ok(records)
}
