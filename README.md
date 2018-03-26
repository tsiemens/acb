# ACB

A golang CLI tool for calculating adjusted cost basis (ACB) and capital gains.
This is primarily designed for Canadians filing tax returns, who have stocks, RSUs, ESPPs, etc.

## Features
- Can compute the total and per-share Adjusted cost base for each stock
- Computes each transaction's capital gain, based on the ACB at that time
- Can perform automatic lookups for the daily CAD/USD exchange rate (from bankofcanada.ca) if needed
- Can accept multiple csv files (eg. one of each year of transactions, or however they are organized)
- Can do computations for multiple securities/symbols in a single execution
- Can apply an "initial" value for each symbol (eg. apply last year's used ACB/stock quanity)

## Example

A csv file like so:
```
security,date,action,shares,amount/share,currency,exchange rate,commission,memo
FOO,2017-01-03,Buy,20,1.5,USD,1.2,1,First buy!
BAR,2017-01-04,Buy,5,50,,,2,
FOO,2017-01-03,Sell,5,1.4,USD,1.2,,:(
BAR,2017-01-04,Sell,1,60,,,,
```
Would yield this:
```
# acb demo.csv
Transactions for FOO
  SECURITY |    DATE    |  TX  |   AMOUNT    | SHARES | AMT/SHARE  | COMMISSION | CAP  GAIN | SHARE BALANCE | ACB +/- | NEW ACB | NEW ACB/SHARE |    MEMO
+----------+------------+------+-------------+--------+------------+------------+-----------+---------------+---------+---------+---------------+------------+
  FOO      | 2017-01-03 | Buy  | $36.00      |     20 | $1.80      | $1.20      |         - |            20 | +$37.20 | $37.20  | $1.86         | First buy!
           |            |      | (30.00 USD) |        | (1.50 USD) | (1.00 USD) |           |               |         |         |               |
+----------+------------+------+-------------+--------+------------+------------+-----------+---------------+---------+---------+---------------+------------+
  FOO      | 2017-01-05 | Sell | $8.40       |      5 | $1.68      |          - | -$0.90    |            15 | -$9.30  | $27.90  | $1.86         | :(
           |            |      | (7.00 USD)  |        | (1.40 USD) |            |           |               |         |         |               |
+----------+------------+------+-------------+--------+------------+------------+-----------+---------------+---------+---------+---------------+------------+
+----------+------------+------+-------------+--------+------------+------------+-----------+---------------+---------+---------+---------------+------------+
                                                                       TOTAL    |  -$0 90   |
                                                                   +------------+-----------+---------------+---------+---------+---------------+------------+

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
