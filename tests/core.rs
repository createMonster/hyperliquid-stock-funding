use hyperliquid_stock_funding::{
    AssetSnapshot, FundingRecord, annualized_rate, dex_for_coin, funding_summary, next_page_start,
    sort_snapshots_by_oi_usd, stock_coins,
};

#[test]
fn stock_coins_keeps_only_stock_categories() {
    let categories = vec![
        ("xyz:AAPL".to_string(), "stocks".to_string()),
        ("xyz:GOLD".to_string(), "commodities".to_string()),
        ("km:TSLA".to_string(), "stocks".to_string()),
    ];

    assert_eq!(stock_coins(categories), vec!["km:TSLA", "xyz:AAPL"]);
}

#[test]
fn funding_summary_sums_rates_and_annualizes_by_window_days() {
    let records = vec![
        FundingRecord::new("xyz:AAPL", 1_700_000_000_000, "0.0001"),
        FundingRecord::new("xyz:AAPL", 1_700_003_600_000, "-0.000025"),
    ];

    let summary = funding_summary("xyz:AAPL", &records, 2.0);

    assert_eq!(summary.samples, 2);
    assert!((summary.total_rate - 0.000075).abs() < 1e-12);
    assert!((summary.annualized_rate - 0.0136875).abs() < 1e-12);
}

#[test]
fn annualized_rate_returns_zero_for_empty_day_window() {
    assert_eq!(annualized_rate(0.01, 0.0), 0.0);
}

#[test]
fn next_page_start_moves_past_last_timestamp() {
    let records = vec![
        FundingRecord::new("xyz:AAPL", 1000, "0.0001"),
        FundingRecord::new("xyz:AAPL", 2000, "0.0002"),
    ];

    assert_eq!(next_page_start(&records), Some(2001));
}

#[test]
fn dex_for_coin_uses_prefix_before_colon() {
    assert_eq!(dex_for_coin("xyz:AAPL"), "xyz");
    assert_eq!(dex_for_coin("BTC"), "");
}

#[test]
fn asset_snapshot_calculates_notional_open_interest() {
    let snapshot = AssetSnapshot::new(
        "xyz:AAPL",
        "10.5",
        Some("200.0"),
        Some("0.0001"),
        Some("25000.0"),
        Some("0.0003"),
    );

    assert!((snapshot.oi_usd() - 2100.0).abs() < 1e-12);
}

#[test]
fn snapshots_sort_by_notional_open_interest_descending() {
    let mut snapshots = vec![
        AssetSnapshot::new("xyz:A", "2.0", Some("10.0"), None, None, None),
        AssetSnapshot::new("xyz:B", "1.0", Some("30.0"), None, None, None),
    ];

    sort_snapshots_by_oi_usd(&mut snapshots);

    assert_eq!(
        snapshots
            .into_iter()
            .map(|snapshot| snapshot.coin)
            .collect::<Vec<_>>(),
        vec!["xyz:B", "xyz:A"]
    );
}
