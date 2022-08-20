# ACB

A golang CLI tool for calculating adjusted cost basis (ACB) and capital gains.
This is primarily designed for Canadians filing tax returns, who have stocks, RSUs, ESPPs, etc.

## Features
- Can compute the total and per-share Adjusted cost base for each stock
- Computes each transaction's capital gain, based on the ACB at that time
- Can perform automatic lookups for the daily CAD/USD exchange rate (from bankofcanada.ca) if needed. This uses the historical noon rates for 2016 and before, and the indicative rate for 2017 and newer.
- By default applies the superficial loss rule to capital losses, when appropriate
- Can accept multiple csv files (eg. one of each year of transactions, or however they are organized)
- Can do computations for multiple securities/symbols in a single execution
- Can apply an "initial" value for each symbol (eg. apply last year's used ACB/stock quanity)

### "Bonus" Script Features
In addition to the main `acb` app, a few extra scripts are provided for convenience. See the `py/` directory (you'll need to run `setup.sh` first).
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
  SECURITY | TRADE DATE | SETTL  DATE |  TX  | AMOUNT  | SHARES | AMT/SHARE |  ACB   | COMMISSION | CAP  GAIN | SHARE BALANCE | ACB +/-  | NEW ACB | NEW ACB/SHARE | MEMO
+----------+------------+-------------+------+---------+--------+-----------+--------+------------+-----------+---------------+----------+---------+---------------+------+
  BAR      | 2017-01-04 | 2017-01-06  | Buy  | $250.00 |      5 | $50.00    |      - | $2.00      |         - |             5 | +$252.00 | $252.00 | $50.40        |
+----------+------------+-------------+------+---------+--------+-----------+--------+------------+-----------+---------------+----------+---------+---------------+------+
  BAR      | 2017-01-06 | 2017-01-08  | Sell | $60.00  |      1 | $60.00    | $50.40 |          - | $9.60     |             4 | -$50.40  | $201.60 | $50.40        |
+----------+------------+-------------+------+---------+--------+-----------+--------+------------+-----------+---------------+----------+---------+---------------+------+
+----------+------------+-------------+------+---------+--------+-----------+--------+------------+-----------+---------------+----------+---------+---------------+------+
                                                                                         TOTAL    |   $9 60   |
                                                                                          2017    |   $9 60   |
                                                                                     +------------+-----------+---------------+----------+---------+---------------+------+

Transactions for FOO
  SECURITY | TRADE DATE | SETTL  DATE |  TX  |   AMOUNT    | SHARES | AMT/SHARE  |  ACB   | COMMISSION |  CAP  GAIN   | SHARE BALANCE |  ACB +/-  | NEW ACB  | NEW ACB/SHARE |              MEMO
+----------+------------+-------------+------+-------------+--------+------------+--------+------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
  FOO      | 2016-01-05 | 2016-01-07  | Buy  | $41.98      |     20 | $2.10      |      - | $1.40      |            - |            20 | +$43.38   | $43.38   | $2.17         | First buy!
           |            |             |      | (30.00 USD) |        | (1.50 USD) |        | (1.00 USD) |              |               |           |          |               |
+----------+------------+-------------+------+-------------+--------+------------+--------+------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
  FOO      | 2017-01-05 | 2017-01-07  | Sell | $8.40       |      5 | $1.68      | $10.84 |          - | -$2.44       |            15 | -$10.84   | $32.53   | $2.17         | :(
           |            |             |      | (7.00 USD)  |        | (1.40 USD) |        |            |              |               |           |          |               |
+----------+------------+-------------+------+-------------+--------+------------+--------+------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
  FOO      | 2017-12-06 | 2017-12-08  | Sell | $38.27      |     15 | $2.55      | $32.53 |          - | $5.74        |             0 | -$32.53   | $0.00    |             - |
           |            |             |      | (30.00 USD) |        | (2.00 USD) |        |            |              |               |           |          |               |
+----------+------------+-------------+------+-------------+--------+------------+--------+------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
  FOO      | 2018-02-05 | 2018-02-07  | Buy  | $36.00      |     20 | $1.80      |      - | $1.20      |            - |            20 | +$37.20   | $37.20   | $1.86         |
           |            |             |      | (30.00 USD) |        | (1.50 USD) |        | (1.00 USD) |              |               |           |          |               |
+----------+------------+-------------+------+-------------+--------+------------+--------+------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
  FOO      | 2018-02-06 | 2018-02-08  | Sell | $17.88      |     10 | $1.79      | $18.60 |          - | $0.00 * (SFL |            10 | -$17.88 * | $19.32 * | $1.93         | Sell to cover taxes
           |            |             |      | (14.90 USD) |        | (1.49 USD) |        |            | -$0.72)      |               | (+$0.72)  | (+$0.72) |               | (superficial loss)
+----------+------------+-------------+------+-------------+--------+------------+--------+------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
+----------+------------+-------------+------+-------------+--------+------------+--------+------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
                                                                                              TOTAL    |    $3 29     |
                                                                                               2016    |    $0 00     |
                                                                                               2017    |    $3 29     |
                                                                                               2018    |    $0 00     |
                                                                                          +------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
 */SFL = Superficial loss adjustment
```

## Installation
Currently, acb must be installed via the golang toolchain.

1\. Set up your go paths (exact directories used below are suggestions only) and environment variables.

```sh
mkdir $HOME/go
export GOPATH=$HOME/go
export PATH=$PATH:$GOPATH/bin
```

2\. Install the golang development tools. This will likely be available through your package manager (eg. apt, brew). [Manual installation instructions](https://golang.org/doc/install) are also available.

3\. Download and install the acb source and dependencies into your GOPATH.

```sh
go get -u -v github.com/tsiemens/acb
```

The `acb` tool should now be ready to use (is installed to $GOPATH/bin/acb)

```sh
acb --help
```

## Uninstall

```sh
# Sanity check what will be done. Don't forget to include the trailing "..." here.
go clean -i -n github.com/tsiemens/acb...
# Clean out installed build files
go clean -i github.com/tsiemens/acb...

# Delete the downloaded source
rm -rf $GOPATH/src/github.com/tsiemens/acb

# You may want to repeat the above steps for other required packages downloaded (shown in go get -v -u), if they are not used by any other top-level package.
```

## Disclaimer
I am not trained in tax law of any form. This tool is provided as is with no
warrenty. Please double check results (enough information should be provided
to do so).

Use at your own risk.
