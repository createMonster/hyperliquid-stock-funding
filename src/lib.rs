use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;
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

#[derive(Debug, Clone, Deserialize)]
struct MetaAndAssetCtxs(MetaResponse, Vec<AssetContext>);

#[derive(Debug, Clone, Deserialize)]
struct MetaResponse {
    universe: Vec<UniverseAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct UniverseAsset {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AssetContext {
    #[serde(rename = "openInterest")]
    open_interest: String,
    #[serde(rename = "oraclePx")]
    oracle_px: Option<String>,
    funding: Option<String>,
    #[serde(rename = "dayNtlVlm")]
    day_ntl_vlm: Option<String>,
    premium: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AssetSnapshot {
    pub coin: String,
    pub open_interest: f64,
    pub oracle_px: Option<f64>,
    pub funding: Option<f64>,
    pub day_ntl_vlm: Option<f64>,
    pub premium: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserFundingEvent {
    pub time: u64,
    pub delta: UserFundingDelta,
}

impl UserFundingEvent {
    pub fn new(
        time: u64,
        coin: impl Into<String>,
        usdc: impl Into<String>,
        szi: impl Into<String>,
        funding_rate: impl Into<String>,
    ) -> Self {
        Self {
            time,
            delta: UserFundingDelta {
                coin: coin.into(),
                usdc: usdc.into(),
                szi: szi.into(),
                funding_rate: funding_rate.into(),
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserFundingDelta {
    pub coin: String,
    pub usdc: String,
    pub szi: String,
    #[serde(rename = "fundingRate")]
    pub funding_rate: String,
}

#[derive(Debug, Clone)]
pub struct DailyUserFunding {
    pub date: String,
    pub total_usdc: f64,
    pub received_usdc: f64,
    pub paid_usdc: f64,
    pub events: usize,
}

impl AssetSnapshot {
    pub fn new(
        coin: impl Into<String>,
        open_interest: &str,
        oracle_px: Option<&str>,
        funding: Option<&str>,
        day_ntl_vlm: Option<&str>,
        premium: Option<&str>,
    ) -> Self {
        Self {
            coin: coin.into(),
            open_interest: parse_f64(open_interest),
            oracle_px: oracle_px.map(parse_f64),
            funding: funding.map(parse_f64),
            day_ntl_vlm: day_ntl_vlm.map(parse_f64),
            premium: premium.map(parse_f64),
        }
    }

    pub fn oi_usd(&self) -> f64 {
        self.open_interest * self.oracle_px.unwrap_or(0.0)
    }
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

pub fn dex_for_coin(coin: &str) -> &str {
    coin.split_once(':').map_or("", |(dex, _)| dex)
}

pub fn sort_snapshots_by_oi_usd(snapshots: &mut [AssetSnapshot]) {
    snapshots.sort_by(|a, b| b.oi_usd().total_cmp(&a.oi_usd()));
}

pub fn daily_user_funding(events: &[UserFundingEvent]) -> Vec<DailyUserFunding> {
    let mut by_date: BTreeMap<String, DailyUserFunding> = BTreeMap::new();

    for event in events {
        let Some(datetime) = DateTime::<Utc>::from_timestamp_millis(event.time as i64) else {
            continue;
        };
        let date = datetime.format("%Y-%m-%d").to_string();
        let usdc = parse_f64(&event.delta.usdc);
        let row = by_date.entry(date.clone()).or_insert(DailyUserFunding {
            date,
            total_usdc: 0.0,
            received_usdc: 0.0,
            paid_usdc: 0.0,
            events: 0,
        });

        row.total_usdc += usdc;
        if usdc >= 0.0 {
            row.received_usdc += usdc;
        } else {
            row.paid_usdc += usdc;
        }
        row.events += 1;
    }

    by_date.into_values().collect()
}

pub fn fetch_stock_coins(client: &Client, api_url: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let categories: Vec<(String, String)> =
        post_info(client, api_url, json!({ "type": "perpCategories" }))?;

    Ok(stock_coins(categories))
}

pub fn fetch_funding_history(
    client: &Client,
    api_url: &str,
    coin: &str,
    start_time: u64,
    end_time: u64,
) -> Result<Vec<FundingRecord>, Box<dyn Error>> {
    let mut start_time = start_time;
    let mut records = Vec::new();

    while start_time <= end_time {
        let page: Vec<FundingRecord> = post_info(
            client,
            api_url,
            json!({
                "type": "fundingHistory",
                "coin": coin,
                "startTime": start_time,
                "endTime": end_time
            }),
        )?;

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

pub fn fetch_asset_snapshots(
    client: &Client,
    api_url: &str,
    coins: &[String],
) -> Result<Vec<AssetSnapshot>, Box<dyn Error>> {
    let requested: BTreeSet<&str> = coins.iter().map(String::as_str).collect();
    let mut coins_by_dex: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for coin in coins {
        coins_by_dex
            .entry(dex_for_coin(coin))
            .or_default()
            .push(coin);
    }

    let mut snapshots = Vec::new();
    for (dex, _) in coins_by_dex {
        let response: MetaAndAssetCtxs = post_info(
            client,
            api_url,
            json!({
                "type": "metaAndAssetCtxs",
                "dex": dex
            }),
        )?;

        let MetaAndAssetCtxs(meta, contexts) = response;
        for (asset, context) in meta.universe.into_iter().zip(contexts) {
            if requested.contains(asset.name.as_str()) {
                snapshots.push(AssetSnapshot::new(
                    asset.name,
                    &context.open_interest,
                    context.oracle_px.as_deref(),
                    context.funding.as_deref(),
                    context.day_ntl_vlm.as_deref(),
                    context.premium.as_deref(),
                ));
            }
        }
    }

    sort_snapshots_by_oi_usd(&mut snapshots);
    Ok(snapshots)
}

pub fn fetch_user_funding(
    client: &Client,
    api_url: &str,
    user: &str,
    start_time: u64,
    end_time: u64,
) -> Result<Vec<UserFundingEvent>, Box<dyn Error>> {
    let mut start_time = start_time;
    let mut events = Vec::new();

    while start_time <= end_time {
        let page: Vec<UserFundingEvent> = post_info(
            client,
            api_url,
            json!({
                "type": "userFunding",
                "user": user,
                "startTime": start_time,
                "endTime": end_time
            }),
        )?;

        if page.is_empty() {
            break;
        }

        let next_start = page.last().map(|event| event.time + 1);
        let is_last_page = page.len() < 500;
        events.extend(page);

        match (is_last_page, next_start) {
            (true, _) | (_, None) => break,
            (false, Some(next_start)) => start_time = next_start,
        }
    }

    Ok(events)
}

fn parse_f64(value: &str) -> f64 {
    value.parse().unwrap_or(0.0)
}

fn post_info<T: DeserializeOwned>(
    client: &Client,
    api_url: &str,
    body: serde_json::Value,
) -> Result<T, Box<dyn Error>> {
    let mut delay = Duration::from_millis(500);
    for attempt in 0..6 {
        let response = client.post(api_url).json(&body).send()?;
        if response.status() == StatusCode::TOO_MANY_REQUESTS && attempt < 5 {
            sleep(delay);
            delay *= 2;
            continue;
        }

        return Ok(response.error_for_status()?.json()?);
    }

    unreachable!()
}
