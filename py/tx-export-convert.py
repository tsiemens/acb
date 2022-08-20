#!/usr/bin/env python3

"""
A convenience script to convert export spreadsheets from brokerages to the ACB
transaction csv format.

Currently only supports Questrade.
"""
import ensure_in_venv

import argparse
import re
import warnings

from txlib import AcbCsvRenderer, Action, Tx

import openpyxl
from openpyxl.workbook.workbook import Workbook as XlWorkbook

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
               trade_date=QuestradeSheet.convert_date_str(get('Transaction Date')),
               settlement_date=QuestradeSheet.convert_date_str(get('Settlement Date')),
               date_and_time=get('Settlement Date'),
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
