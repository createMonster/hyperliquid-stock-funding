# Hyperliquid Stock Funding

A small Rust CLI for calculating total and annualized funding rates for Hyperliquid stock perpetuals.

The tool reads Hyperliquid's public `info` API, discovers markets categorized as `stocks`, fetches historical funding rates, and ranks the results by annualized funding.

## Install

```bash
cargo install --git https://github.com/createMonster/hyperliquid-stock-funding
```

Or run locally:

```bash
cargo run --release -- --days 30
```

## Usage

Scan all Hyperliquid stock perps:

```bash
hl-stock-funding --days 30
```

Check specific markets:

```bash
hl-stock-funding xyz:AAPL km:TSLA flx:NVDA --days 7
```

Show current open interest for all stock perps:

```bash
hl-stock-funding oi
```

Show current open interest for specific markets:

```bash
hl-stock-funding oi xyz:AAPL km:TSLA flx:NVDA
```

Example output:

```text
coin                samples          total     annualized
xyz:AAPL                720        -0.2500%        -3.04%
km:TSLA                 720         0.1800%         2.19%
```

## Calculation

```text
total funding rate = sum(hourly funding rates in the lookback window)
annualized rate    = total funding rate * 365 / days
```

Funding history is fetched from:

```text
POST https://api.hyperliquid.xyz/info
```
