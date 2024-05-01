#!/usr/bin/env python3
"""
For a CSV file, find all cells that have a "$" in them and attempt to strip the
currency symbol and all punctuation. Then round the output to two decimal places.

The modified CSV file is printed to stdout.
"""
import re
import sys
from csv import reader, writer
from datetime import datetime
from decimal import Decimal, ROUND_HALF_EVEN

DATE_FMT = "%Y-%m-%d"


def convert(v: str) -> str | Decimal:
    try:
        return str(datetime.strptime(v, DATE_FMT))
    except ValueError:
        pass

    if not v.strip().startswith("$"):
        return v

    try:
        d = Decimal(re.sub(r"[$, ]", "", v))
        return d.quantize(Decimal("0.01"), rounding=ROUND_HALF_EVEN)
    except ValueError:
        return v


def main():
    rows = []
    with open(sys.argv[1]) as fp:
        rd = reader(fp)

        for row in rd:
            rows.append([convert(i) for i in row])

    wr = writer(sys.stdout)
    wr.writerows(rows)


if __name__ == "__main__":
    main()
