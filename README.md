# Hyperliquid Stock Funding

A small Rust CLI for calculating total and annualized funding rates for Hyperliquid stock perpetuals.

The tool reads Hyperliquid's public `info` API, discovers markets categorized as `stocks`, fetches historical funding rates, and ranks the results by annualized funding.

The CLI retries rate-limited API requests with exponential backoff.

## Install

```bash
cargo install --git https://github.com/createMonster/hyperliquid-stock-funding
```

Or run locally:

```bash
cargo run --release -- --days 30
```

## Configuration

Wallet-level funding commands read `HYPERLIQUID_WALLET` from a local `.env` file:

```bash
cp .env.example .env
# edit .env and set HYPERLIQUID_WALLET
```

The `.env` file is ignored by git.

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

Show daily funding received or paid by a wallet:

```bash
hl-stock-funding wallet-funding --days 30
```

You can also pass a wallet explicitly:

```bash
hl-stock-funding wallet-funding --wallet 0x... --days 30
```

Example output:

```text
coin                samples          total     annualized
xyz:AAPL                720        -0.2500%        -3.04%
km:TSLA                 720         0.1800%         2.19%
```

Wallet funding output:

```text
date           total_usdc     received         paid   events
2026-06-09        29.5608      31.8081      -2.2472      120
```

## Calculation

```text
total funding rate = sum(hourly funding rates in the lookback window)
annualized rate    = total funding rate * 365 / days
wallet net funding = received funding + paid funding
```

## Interpretation

High funding is more interesting when it is supported by rising open interest and enough trading volume. A simple working hypothesis:

```text
high funding + rising OI = trend demand may still be pushing the market
high funding + falling OI = the move may be a late squeeze or unwind
high funding + weak volume = the signal is less reliable
```

The `oi` command shows the current OI snapshot and 24h notional volume. To judge whether OI is rising, compare repeated snapshots over time.

Funding history is fetched from:

```text
POST https://api.hyperliquid.xyz/info
```
