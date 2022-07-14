#!/usr/bin/env python3

"""
A convenience script to convert export spreadsheets from brokerages to the ACB
transaction csv format.

Currently only supports Questrade.
"""

import argparse
import csv
from dataclasses import dataclass
from enum import Enum
import re
import sys
import warnings

import openpyxl
from openpyxl.workbook.workbook import Workbook as XlWorkbook

# Output format:
# Security,Date,Action,Amount/Share,Shares,Commission,Currency,Memo,Exchange Rate
# SPXT,2021-01-19,Buy,65.9,150,0,USD,Settlement date. Trade date:14-01-21,

class Action(Enum):
   BUY = 'BUY'
   SELL = 'SELL'

@dataclass
class Tx:
   security: str
   settlement_date: str # Settlement date
   date_and_time: str # Just used for sorting
   transaction_date: str
   action: Action
   amount_per_share: float
   num_shares: int
   commission: float
   currency: str
   memo: str
   exchange_rate: float

class AcbCsvRenderer:
   def __init__(self):
      self.txs = []

   def sort_txs(self):
      self.txs = sorted(self.txs, key=lambda t: t.date_and_time)

   def rows(self):
      for tx in self.txs:
         yield (tx.security, tx.transaction_date, tx.settlement_date, str(tx.action),
                str(tx.amount_per_share),
                str(tx.num_shares), str(tx.commission), tx.currency, tx.memo,
                str(tx.exchange_rate) if tx.exchange_rate else '')

   def header_row(self):
      return ['Security', 'Trade Date', 'Settlement Date', 'Action', 'Amount/Share',
              'Shares', 'Commission', 'Currency', 'Memo', 'Exchange Rate']

   def render_csv(self):
      writer = csv.writer(sys.stdout, delimiter=',', quoting=csv.QUOTE_MINIMAL)
      writer.writerow(self.header_row())
      for row in self.rows():
         writer.writerow(row)

   def render_table(self):
      all_rows = [self.header_row()] + [r for r in self.rows()]
      col_widths = [1] * len(all_rows[0])
      # Determine the max col widths
      for row in all_rows:
         for i, col in enumerate(row):
            col_widths[i] = max(col_widths[i], len(col))

      fmt = '|'.join('{:%d}' % w for w in col_widths)
      for row in all_rows:
         print(fmt.format(*row))

class QuestradeSheet(AcbCsvRenderer):
   # Column names:
   #  'Transaction Date', 'Settlement Date', 'Action''Symbol', 'Description',
   #  'Quantity', 'Price', 'Gross Amount', 'Commission', 'Net Amount', 'Currency',
   #  'Account #', 'Activity Type', 'Account Type'

   symbol_aliases = {
      # symbol : (alias_to, AKA)
      'H038778': ('DLR.TO', 'DLR.U.TO'),
   }

   def __init__(self, xl_wb: XlWorkbook):
      super().__init__()
      ignored_actions = {'DIV', 'BRW', 'TFI', 'DEP', None}
      allowed_actions = {'Buy', 'Sell'}

      header_row = None
      col_name_to_index = {}
      sheet = next(iter(xl_wb.worksheets))
      for row in sheet.rows:
         if header_row is None:
            header_row = [c.value for c in row]
            col_name_to_index = {c: i for i, c in enumerate(header_row)}
         else:
            def get(name):
               return row[col_name_to_index[name]].value

            action_str = get('Action')
            if (action_str not in allowed_actions and
                action_str not in ignored_actions):
               raise Exception(f"Unrecognized transaction action {repr(action_str)}"
                               f" in row {repr([c.value for c in row])}")
            if action_str in ignored_actions:
               continue

            action = Action(action_str.upper())
            symbol = get('Symbol')
            orig_symbol_note = ''
            if symbol in QuestradeSheet.symbol_aliases:
               orig_symbol = symbol
               symbol, aka = QuestradeSheet.symbol_aliases[symbol]
               orig_symbol_note = f"{orig_symbol} AKA {aka}. "

            tx = Tx(
               security=symbol,
               settlement_date=QuestradeSheet.convert_date_str(get('Settlement Date')),
               date_and_time=get('Settlement Date'),
               transaction_date=QuestradeSheet.convert_date_str(get('Transaction Date')),
               action=action.value,
               amount_per_share=float(get('Price')),
               num_shares=QuestradeSheet.get_quantity_int(get('Quantity'),
                                                          action),
               commission=abs(float(get('Commission'))),
               currency=get('Currency'),
               memo=orig_symbol_note,
               exchange_rate=None,
            )
            self.txs.append(tx)

   date_regexp = re.compile(r'^\d{4}-\d{2}-\d{2}')

   @staticmethod
   def convert_date_str(qt_date: str) -> str:
      """Returns a yyyy-mm-dd formatted date string"""
      m = QuestradeSheet.date_regexp.match(qt_date)
      if m:
         return m.group(0)
      else:
         raise Exception(f"Could not parse date from {repr(qt_date)}")

   @staticmethod
   def get_quantity_int(quant_str: str, action: Action) -> int:
      quant = int(float(quant_str))
      if action == Action.BUY:
         return quant
      else:
         # QT's table shows sold shares as a negative quantity.
         return -1 * quant

def dump_xl_workbook(wb: XlWorkbook):
   for sheet in wb.worksheets:
      print(f'Sheet {sheet.title}:')
      for row in sheet.rows:
         for something in row:
            print(repr(something.value), end='')
         print()

def read_xl_file(fname) -> AcbCsvRenderer:
   with warnings.catch_warnings(record=True):
      warnings.simplefilter("always")
      wb = openpyxl.load_workbook(fname, read_only=True, data_only=True)
   return QuestradeSheet(wb)

def main():
   parser = argparse.ArgumentParser(description=__doc__)
   parser.add_argument('export_file',
         help="Table file exported from your brokerage platform. "
         "A .xlsx for Questrade")
   parser.add_argument('--no-sort', action='store_true')
   parser.add_argument('--broker', '-b', default='questrade',
                       choices=( 'questrade', ))
   parser.add_argument('--usd-exchange-rate', type=float,
         help="Specify an exchange rate to use (rates for more recent"
         " transactions may not be posted yet)")
   parser.add_argument('--pretty', action='store_true')
   args = parser.parse_args()

   sheet = read_xl_file(args.export_file)
   if not args.no_sort:
      sheet.sort_txs()

   if args.usd_exchange_rate is not None:
      for tx in sheet.txs:
         if tx.currency == 'USD':
            tx.exchange_rate = args.usd_exchange_rate

   if args.pretty:
      sheet.render_table()
   else:
      sheet.render_csv()

if __name__ == '__main__':
   main()
