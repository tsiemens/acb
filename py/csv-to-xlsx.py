#!/usr/bin/env python3

from posixpath import splitext
from typing import Any, List, Callable, Tuple, Union
import openpyxl
from datetime import datetime
from openpyxl.worksheet.worksheet import Worksheet
from openpyxl.cell import Cell
import sys
from os.path import basename
from csv import reader

DATE_FMT = "%Y-%m-%d"

CONVERTER = Callable[[str], Tuple[Any, str]]


def maybe_number(val: str) -> Tuple[Union[int, str], str]:
    if val.isnumeric():
        return int(val), ""
    return val, ""


def convert_date(val: str) -> Tuple[Union[datetime, str], str]:
    try:
        return datetime.strptime(val, DATE_FMT), "yyyy-mm-dd"
    except ValueError:
        return val, ""


def get_converter(header: List[str]) -> List[CONVERTER]:
    converter: List[CONVERTER] = []
    for col in header:
        if col.lower().endswith("date"):
            converter.append(convert_date)
        else:
            converter.append(maybe_number)
    return converter


def handle_file(wb: openpyxl.Workbook, name: str, sheet_num: int):
    header = None
    converter = None
    with open(name) as fp:
        rd = reader(fp)
        title = splitext(basename(name))[0]
        sheet: Worksheet = wb.create_sheet(title=title, index=sheet_num)
        row_num = 0
        for row in rd:
            row_num += 1
            if header is None:
                header = row
                converter = get_converter(header)
                sheet.append(row)
            else:
                col_num = 0
                assert converter
                cells = []
                for conv, val in zip(converter, row):
                    col_num += 1
                    val, fmt = conv(val)
                    c = Cell(sheet, row_num, col_num, value=val)
                    if fmt:
                        c.number_format = fmt
                    cells.append(c)
                sheet.append(cells)
        for rowNum in range(row_num):
            sheet.row_dimensions[rowNum + 1].height = 60


def main():
    files = sys.argv[1:]
    if not files:
        return

    wb = openpyxl.Workbook(write_only=True)
    for idx, fn in enumerate(sys.argv[1:]):
        handle_file(wb, fn, idx + 1)
    wb.save("out.xlsx")


if __name__ == "__main__":
    main()
