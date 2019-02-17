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

## Example

A csv file like so:
```
security,date,action,shares,amount/share,currency,exchange rate,commission,memo
FOO,2016-01-05,Buy,20,1.5,USD,,1,First buy!
BAR,2017-01-04,Buy,5,50,,,2,
FOO,2017-01-05,Sell,5,1.4,USD,1.2,,:(
BAR,2017-01-06,Sell,1,60,,,,
FOO,2017-12-06,Sell,15,2,USD,,,
FOO,2018-02-05,Buy,20,1.5,USD,1.2,1,
FOO,2018-02-06,Sell,10,1.49,USD,1.2,,Sell to cover taxes (superficial loss)
```
Would yield this:
```
# acb demo.csv
Transactions for FOO
  SECURITY |    DATE    |  TX  |   AMOUNT    | SHARES | AMT/SHARE  | COMMISSION |  CAP  GAIN   | SHARE BALANCE |  ACB +/-  | NEW ACB  | NEW ACB/SHARE |              MEMO
+----------+------------+------+-------------+--------+------------+------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
  FOO      | 2016-01-05 | Buy  | $41.98      |     20 | $2.10      | $1.40      |            - |            20 | +$43.38   | $43.38   | $2.17         | First buy!
           |            |      | (30.00 USD) |        | (1.50 USD) | (1.00 USD) |              |               |           |          |               |
+----------+------------+------+-------------+--------+------------+------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
  FOO      | 2017-01-05 | Sell | $8.40       |      5 | $1.68      |          - | -$2.44       |            15 | -$10.84   | $32.53   | $2.17         | :(
           |            |      | (7.00 USD)  |        | (1.40 USD) |            |              |               |           |          |               |
+----------+------------+------+-------------+--------+------------+------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
  FOO      | 2017-12-06 | Sell | $38.27      |     15 | $2.55      |          - | $5.74        |             0 | -$32.53   | $0.00    |             - |
           |            |      | (30.00 USD) |        | (2.00 USD) |            |              |               |           |          |               |
+----------+------------+------+-------------+--------+------------+------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
  FOO      | 2018-02-05 | Buy  | $36.00      |     20 | $1.80      | $1.20      |            - |            20 | +$37.20   | $37.20   | $1.86         |
           |            |      | (30.00 USD) |        | (1.50 USD) | (1.00 USD) |              |               |           |          |               |
+----------+------------+------+-------------+--------+------------+------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
  FOO      | 2018-02-06 | Sell | $17.88      |     10 | $1.79      |          - | $0.00 * (was |            10 | -$17.88 * | $19.32 * | $1.93         | Sell to cover taxes
           |            |      | (14.90 USD) |        | (1.49 USD) |            | -$0.72)      |               | (+$0.72)  | (+$0.72) |               | (superficial loss)
+----------+------------+------+-------------+--------+------------+------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
+----------+------------+------+-------------+--------+------------+------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
                                                                       TOTAL    |    $3 29     |
                                                                   +------------+--------------+---------------+-----------+----------+---------------+--------------------------------+
 * = Superficial loss adjustment

Transactions for BAR
  SECURITY |    DATE    |  TX  | AMOUNT  | SHARES | AMT/SHARE | COMMISSION | CAP  GAIN | SHARE BALANCE | ACB +/-  | NEW ACB | NEW ACB/SHARE | MEMO
+----------+------------+------+---------+--------+-----------+------------+-----------+---------------+----------+---------+---------------+------+
  BAR      | 2017-01-04 | Buy  | $250.00 |      5 | $50.00    | $2.00      |         - |             5 | +$252.00 | $252.00 | $50.40        |
+----------+------------+------+---------+--------+-----------+------------+-----------+---------------+----------+---------+---------------+------+
  BAR      | 2017-01-06 | Sell | $60.00  |      1 | $60.00    |          - | $9.60     |             4 | -$50.40  | $201.60 | $50.40        |
+----------+------------+------+---------+--------+-----------+------------+-----------+---------------+----------+---------+---------------+------+
+----------+------------+------+---------+--------+-----------+------------+-----------+---------------+----------+---------+---------------+------+
                                                                  TOTAL    |   $9 60   |
                                                              +------------+-----------+---------------+----------+---------+---------------+------+
```

## Set Up/Development
```
make getdeps
make
make test

export PATH=$PATH:$(pwd)/bld
acb ...
```

## Disclaimer
I am not trained in tax law of any form. This tool is provided as is with no
warrenty. Please double check results (enough information should be provided
to do so).

Use at your own risk.
