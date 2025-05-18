# ACB

[![badges](https://github.com/tsiemens/acb/workflows/Rust/badge.svg)](https://github.com/tsiemens/acb/actions)
[![license](https://img.shields.io/badge/License-MIT-purple.svg)](LICENSE)

A CLI tool for calculating adjusted cost basis (ACB) and capital gains.
This is primarily designed for Canadians filing tax returns, who have stocks, RSUs, ESPPs, etc.

## Features
- Can compute the total and per-share Adjusted cost base for each stock
- Computes each transaction's capital gain, based on the ACB at that time
- Can perform automatic lookups for the daily CAD/USD exchange rate (from bankofcanada.ca) if needed. This uses the historical noon rates for 2016 and before, and the indicative rate for 2017 and newer.
- By default applies the superficial loss rule to capital losses, when appropriate
- Can accept multiple csv files (eg. one of each year of transactions, or however they are organized)
- Can do computations for multiple securities/symbols in a single execution
- Can apply an "initial" value for each symbol (eg. apply last year's used ACB/stock quanity) ("Default" affiliate only).
- Can emit a "summary" CSV, which can compact a large number of historical transactions into just a few, which preserve a record of yearly capital gains.
- Supports multiple "affiliated persons", or "affiliates". Eg. your spouse or your registered accounts (RRSPs, TFSAs, etc.).

### "Bonus" Script Features
In addition to the main `acb` app, a few extra utilities are provided for convenience.
Most/all of the complex scripts are also compiled into `target/...` or your cargo bin (see installation below), though some simpler scripts (without any dependencies) are in the `py/` directory.
- tx-export-convert: Convert exported transaction spreadsheets to acb-compatible csv files (Questrade-only for now)
- etrade-plan-pdf-tx-extract: Generate acb csv files from ETRADE stock plan PDFs.

## Example

A csv file like so:
```
security,trade date,settlement date,action,shares,amount/share,currency,exchange rate,commission,memo
FOO,2016-01-05,2016-01-07,Buy,20,1.5,USD,,1,First buy!
BAR,2017-01-04,2017-01-06,Buy,5,50,,,2,
FOO,2017-01-05,2017-01-07,Sell,5,1.4,USD,1.2,,:(
BAR,2017-01-06,2017-01-08,Sell,1,60,,,,
FOO,2017-12-06,2017-12-08,Sell,15,2,USD,,,
FOO,2018-02-05,2018-02-07,Buy,20,1.5,USD,1.2,1,
FOO,2018-02-06,2018-02-08,Sell,10,1.49,USD,1.2,,Sell to cover taxes (superficial loss)
```
Would yield this:
```
# acb demo.csv
Transactions for BAR
  SECURITY | TRADE DATE | SETTL  DATE |  TX  | AMOUNT  | SHARES | AMT/SHARE |  ACB   | COMMISSION | CAP  GAIN | SHARE BALANCE | ACB +/-  | NEW ACB | NEW ACB/SHARE | AFFILIATE | MEMO
-----------+------------+-------------+------+---------+--------+-----------+--------+------------+-----------+---------------+----------+---------+---------------+-----------+-------
  BAR      | 2017-01-04 | 2017-01-06  | Buy  | $250.00 |      5 | $50.00    | -      | $2.00      | -         |             5 | +$252.00 | $252.00 | $50.40        | Default   |
-----------+------------+-------------+------+---------+--------+-----------+--------+------------+-----------+---------------+----------+---------+---------------+-----------+-------
  BAR      | 2017-01-06 | 2017-01-08  | Sell | $60.00  |      1 | $60.00    | $50.40 | -          | $9.60     |             4 | -$50.40  | $201.60 | $50.40        | Default   |
-----------+------------+-------------+------+---------+--------+-----------+--------+------------+-----------+---------------+----------+---------+---------------+-----------+-------
-----------+------------+-------------+------+---------+--------+-----------+--------+------------+-----------+---------------+----------+---------+---------------+-----------+-------
                                                                                         TOTAL    |   $9.60   |
                                                                                          2017    |   $9.60   |
                                                                                     -------------+-----------+---------------+----------+---------+---------------+-----------+-------

Transactions for FOO
  SECURITY | TRADE DATE | SETTL  DATE |  TX  |   AMOUNT    | SHARES | AMT/SHARE  |  ACB   | COMMISSION |      CAP  GAIN      | SHARE BALANCE | ACB +/- | NEW ACB | NEW ACB/SHARE | AFFILIATE |              MEMO
-----------+------------+-------------+------+-------------+--------+------------+--------+------------+---------------------+---------------+---------+---------+---------------+-----------+---------------------------------
  FOO      | 2016-01-05 | 2016-01-07  | Buy  | $41.98      |     20 | $2.10      | -      | $1.40      | -                   |            20 | +$43.38 | $43.38  | $2.17         | Default   | First buy!
           |            |             |      | (30.00 USD) |        | (1.50 USD) |        | (1.00 USD) |                     |               |         |         |               |           |
-----------+------------+-------------+------+-------------+--------+------------+--------+------------+---------------------+---------------+---------+---------+---------------+-----------+---------------------------------
  FOO      | 2017-01-05 | 2017-01-07  | Sell | $8.40       |      5 | $1.68      | $10.84 | -          | -$2.44              |            15 | -$10.84 | $32.53  | $2.17         | Default   | :(
           |            |             |      | (7.00 USD)  |        | (1.40 USD) |        |            |                     |               |         |         |               |           |
-----------+------------+-------------+------+-------------+--------+------------+--------+------------+---------------------+---------------+---------+---------+---------------+-----------+---------------------------------
  FOO      | 2017-12-06 | 2017-12-08  | Sell | $38.27      |     15 | $2.55      | $32.53 | -          | $5.74               |             0 | -$32.53 | $0.00   | -             | Default   |
           |            |             |      | (30.00 USD) |        | (2.00 USD) |        |            |                     |               |         |         |               |           |
-----------+------------+-------------+------+-------------+--------+------------+--------+------------+---------------------+---------------+---------+---------+---------------+-----------+---------------------------------
  FOO      | 2018-02-05 | 2018-02-07  | Buy  | $36.00      |     20 | $1.80      | -      | $1.20      | -                   |            20 | +$37.20 | $37.20  | $1.86         | Default   |
           |            |             |      | (30.00 USD) |        | (1.50 USD) |        | (1.00 USD) |                     |               |         |         |               |           |
-----------+------------+-------------+------+-------------+--------+------------+--------+------------+---------------------+---------------+---------+---------+---------------+-----------+---------------------------------
  FOO      | 2018-02-06 | 2018-02-08  | Sell | $17.88      |     10 | $1.79      | $18.60 | -          | $0.00 * (SfL        |            10 | -$18.60 | $18.60  | $1.86         | Default   | Sell to cover taxes
           |            |             |      | (14.90 USD) |        | (1.49 USD) |        |            | -$0.72; 10/10)      |               |         |         |               |           | (superficial loss)
-----------+------------+-------------+------+-------------+--------+------------+--------+------------+---------------------+---------------+---------+---------+---------------+-----------+---------------------------------
  FOO      | 2018-02-06 | 2018-02-08  | SfLA | $0.72       |     10 | $0.07      | -      | -          | -                   |            10 | +$0.72  | $19.32  | $1.93         | Default   | automatic SfL ACB adjustment
           |            |             |      | (0.72 CAD)  |        | (0.07 CAD) |        |            |                     |               |         |         |               |           |
-----------+------------+-------------+------+-------------+--------+------------+--------+------------+---------------------+---------------+---------+---------+---------------+-----------+---------------------------------
-----------+------------+-------------+------+-------------+--------+------------+--------+------------+---------------------+---------------+---------+---------+---------------+-----------+---------------------------------
                                                                                              TOTAL    |        $3.29        |
                                                                                               2016    |        $0.00        |
                                                                                               2017    |        $3.29        |
                                                                                               2018    |        $0.00        |
                                                                                          -------------+---------------------+---------------+---------+---------+---------------+-----------+---------------------------------
 */SfL = Superficial loss adjustment

Aggregate Gains
       YEAR       | CAPITAL GAINS
------------------+----------------
             2016 | $0.00
------------------+----------------
             2017 | $12.89
------------------+----------------
             2018 | $0.00
------------------+----------------
  Since inception | $12.89
------------------+----------------
```

## Installation/Build
### Requirements
You'll need `cargo` to either build or install the app. Installation is faily simple:

https://doc.rust-lang.org/cargo/getting-started/installation.html

If you want to build the web interface, you'll need to install `npm`, which should be available from your OS's package manager.

### Installation Only

If you just want the `acb` binary, you can install it by running:
```
cargo install --git https://github.com/tsiemens/acb.git
```
This will pull, build, and install it in the cargo binaries directory (`~/.cargo/bin/`). Just make sure this is in your `PATH`.

Check that it is running as expected:
```sh
acb --help
```

Otherwise, see build instructions below.

### Building
For convenience, `make` aliases have been added to wrap the common `cargo` commands (See `Makefile`).
```
# Build a debug version of just the `acb` binary
make
# Build a release optimized version of the `acb` binary
make release
# Build everything, and run tests
make all
```

The executables will go into the `target/debug` or `target/release` directory.

### Testing
```
make test
# OR
cargo t <options>
```

## Uninstall

```sh
cargo uninstall acb
# If you want to clean up all of the temporary files used to build,
# and you have no other rust projects using them as a cache still.
rm -r ~/.cargo/registry
rm -r ~/.cargo/git
```

## Deprecated Golang Implementation
If you have used `acb` before 2024, you may notice that the toolchain has changed.
Due to concerns about the stability and correctness guarantees that could be
reasonably achieved in the old implementation
(see [this issue](https://github.com/tsiemens/acb/issues/21)), I decided to
rewrite the application in rust, which essentially solved all of those concerns.

However, there are some potential differences besides the build and install process,
notably that *Windows support* has the potential to be less stable than it
used to be due to requiring more special treatment. If you have issues with the new implementation, the last version written in golang is available still at
[this branch](https://github.com/tsiemens/acb/tree/golang), however it will
be very unlikely to see any updates. I will try to fix any reported issues
on Windows in the new version as they come up.

## Disclaimer
I am not trained in tax law of any form. This tool is provided as is with no
warrenty. Please double check results (enough information should be provided
to do so).

Use at your own risk.
