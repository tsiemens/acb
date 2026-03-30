# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Project Is

`acb` is a Canadian Adjusted Cost Base (ACB) calculator for tracking capital gains/losses on securities for tax purposes. It processes transaction CSV files and computes ACB, superficial losses, and annual capital gains.

## Build & Test Commands

```sh
make              # cargo build (debug)
make release      # cargo build --release
make test         # cargo test (all tests including integration)
make test-unit    # cargo test --lib --bins (excludes tests/ directory)
make rustfmt      # format all Rust source files (run before PRs)
make install      # cargo install --path .
```

To run a single test:
```sh
cargo test test_name
cargo test test_name -- --nocapture   # show stdout
VERBOSE=1 cargo test test_name        # some tests use VERBOSE env var
```

Web/WASM build:
```sh
make web          # builds acb_wasm + www
make acb_wasm     # WASM package only (uses wasm-pack)
make www          # TypeScript/Vite frontend only
```

Do NOT run the system `npm`. Only either run the `make` commands in www/, or `www/scripts/npm`, which should run the correct version.

Python utilities (in `py/`):
```sh
make test-py      # runs pytest in py/
```

## Architecture

### Core Library (`src/`)

- **`src/portfolio/`** — Core ACB calculation engine
  - `model/` — Fundamental types: `Tx`, `TxAction` (Buy/Sell/Roc/Split/etc.), `TxDelta`, `CsvTx`, `Affiliate`, `Currency`, `Security` (alias for `String`)
  - `bookkeeping/` — ACB computation: `txs_to_delta_list()`, superficial loss detection, cumulative gains
  - `io/tx_csv.rs` — CSV parsing (`parse_tx_csv`) and writing (`write_txs_to_csv`)
  - `render.rs` — Converts delta lists into `RenderTable` (generic table model)
  - `splits.rs` — Stock split/reverse-split handling
  - `summary.rs` — Summary mode: aggregates positions as of a given date

- **`src/fx/`** — FX rate loading: remote fetching (Bank of Canada API), file-based caching, in-memory cache for tests

- **`src/app/`** — High-level application entry points
  - `approot.rs` — Main app functions: `run_acb_app_to_render_model`, `run_acb_app_to_writer`, `run_acb_app_to_console`, summary mode equivalents
  - `outfmt/` — Output formatters implementing `AcbWriter` trait: `TextWriter` (console tables), `CsvWriter`/`CsvZipWriter` (CSV output)

- **`src/peripheral/`** — Broker-specific integrations and file format adapters
  - `broker/etrade.rs`, `broker/questrade.rs`, `broker/rbc_di.rs` — Broker transaction parsers
  - `excel.rs`, `tx_export_convert_impl.rs` — Excel/.xlsx import
  - `etrade_plan_pdf_tx_extract_impl.rs`, `questrade_statement_fmv_impl.rs` — PDF extraction

- **`src/bin/`** — CLI entry points: `acb` (main), `pdf-text`, `tx-export-convert`, `csv-to-xlsx`, `questrade-statement-fmv`, `etrade-plan-pdf-tx-extract`

- **`src/cmd.rs`** — CLI argument parsing (clap)

### WASM & Web (`acb_wasm/`, `www/`)

`acb_wasm/` wraps the core library as a `cdylib` for WebAssembly using `wasm-bindgen`. The web frontend in `www/` is TypeScript + Vite, with the WASM package symlinked at `www/src/pkg`.

### Integration Tests (`tests/`)

Integration tests live in `tests/` and test end-to-end rendering, FX rate loading, file format conversion, and sample file processing.

## Key Conventions

- **Version scheme**: `0.YY.MM[.i]` (year/month-based, major stays at 0). Defined in `src/app.rs` as `ACB_APP_VERSION` — this overrides `Cargo.toml` version.
- **Cargo features**: `default = ["cliapp", "pdf_parse", "xlsx_read", "xlsx_write"]`; `wasm` disables non-WASM-compatible deps; `testlib` exposes test helpers.
- **Decimal arithmetic**: Uses `rust_decimal` throughout. Custom wrapper types (`PosDecimal`, `GreaterEqualZeroDecimal`, etc.) in `src/util/decimal.rs` enforce invariants.
- **Async**: Uses `async-std` (not tokio). Tests use `async_std::task::block_on`.
- **Formatting**: `.rustfmt.toml` configures rustfmt. Run `make rustfmt` before submitting PRs. Some line-width allowances are made for unit tests.
