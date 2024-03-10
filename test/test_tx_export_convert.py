#!/usr/bin/env python3

import csv
import io
import os
import re
import subprocess

import pytest

script_path = os.path.join(os.path.dirname(__file__), '../py/tx-export-convert')
test_file_path = os.path.join(os.path.dirname(__file__), 'QT_Test_Export.xlsx')

def run(options: list[str]):
   cmd = [script_path, test_file_path] + options
   return subprocess.run(cmd, capture_output=True)

def run_and_get_lines(options: list[str]) -> tuple[list[str], list[str]]:
   out = run(options)
   assert out.returncode == 0
   errs = []
   if out.stderr != b'':
      err_lines = out.stderr.decode().splitlines()
      assert err_lines[1] == "Errors:"
      errs = err_lines[2:]

   lines = out.stdout.decode().splitlines()
   return lines, errs

def align_csv_output(output) -> str:
   lines = [l for l in output.splitlines() if l]
   csv_rows = [r for r in csv.reader(lines)]
   max_col_lens = [0] * max(len(row) for row in csv_rows)
   for row in csv_rows:
      for i, col in enumerate(row):
         max_col_lens[i] = max(max_col_lens[i], len(col))

   stream = io.StringIO()
   writer = csv.writer(stream)
   padded_rows = []
   for row in csv_rows:
      padded_row = [
            col + (' ' * (max_col_lens[i] - len(col)))
            for i, col in enumerate(row)
         ]
      writer.writerow(padded_row)

   return stream.getvalue()

def padded_csv_text_to_rows(padded_csv_text):
   lines = [l for l in padded_csv_text.splitlines() if l]
   csv_rows = [r for r in csv.reader(lines)]
   trimmed_rows = [
         tuple(col.strip() for col in row)
         for row in csv_rows
      ]
   return trimmed_rows

def error_text_to_errors(error_text):
   return [l for l in error_text.splitlines() if l]

def do_emit_aligned_csv_output():
   val = align_csv_output("""
Security,Trade Date,Settlement Date,Action,Amount/Share,Shares,Commission,Currency,Affiliate,Memo,Exchange Rate
USD.FX,2023-02-05,2023-02-05,BUY,0.01,10000,0.0,USD,,Questrade Individual margin 10000003; FXT,1.3
USD.FX,2023-02-05,2023-02-05,SELL,0.01,20000,0.0,USD,,Questrade Individual margin 10000003; FXT,1.25
""")
   breakpoint()
   print(val)

# Uncomment this to generate nice test rows.
#  do_emit_aligned_csv_output()

def verify_csv(lines: list[str], exp_rows: list):
   reader = csv.reader(lines)
   for row, exp_row in zip(csv.reader(lines), exp_rows):
      assert tuple(row) == tuple(exp_row)
   assert len(lines) == len(exp_rows)

def verify_csv_from_text(lines: list[str], exp_csv: str):
   exp_rows = padded_csv_text_to_rows(exp_csv)
   verify_csv(lines, exp_rows)

header = [
      'Security', 'Trade Date', 'Settlement Date', 'Action', 'Amount/Share',
      'Shares', 'Commission', 'Currency', 'Affiliate', 'Memo', 'Exchange Rate']

def test_txs_basic_and_ignored_actions():
   lines, errs = run_and_get_lines(['-a', '.'])

   exp_csv = """
Security,Trade Date,Settlement Date,Action,Amount/Share,Shares,Commission,Currency,Affiliate,Memo                                                     ,Exchange Rate
CCO     ,2023-01-04,2023-01-05     ,BUY   ,10.0        ,2     ,3.99      ,CAD     ,(R)      ,Questrade Individual TFSA 10000001                       ,
DLR.TO  ,2023-01-06,2023-01-07     ,BUY   ,1.0         ,3     ,0.0       ,CAD     ,(R)      ,Questrade Individual RRSP 10000002                       ,
CCO     ,2023-01-08,2023-01-09     ,BUY   ,10.0        ,4     ,1.0       ,CAD     ,         ,Questrade Individual margin 10000003                     ,
USD.FX  ,2023-01-10,2023-01-10     ,SELL  ,0.01        ,5100  ,0.0       ,USD     ,(R)      ,Questrade Individual TFSA 10000001; from DLR.TO BUY      ,
DLR.TO  ,2023-01-10,2023-01-11     ,BUY   ,10.0        ,5     ,1.0       ,USD     ,(R)      ,Questrade Individual TFSA 10000001; H038778 AKA DLR.U.TO.,
USD.FX  ,2023-01-12,2023-01-12     ,SELL  ,0.01        ,6000  ,0.0       ,USD     ,(R)      ,Questrade Individual RRSP 10000002; from UCO BUY         ,
UCO     ,2023-01-12,2023-01-13     ,BUY   ,10.0        ,6     ,0.0       ,USD     ,(R)      ,Questrade Individual RRSP 10000002                       ,
USD.FX  ,2023-01-14,2023-01-14     ,SELL  ,0.01        ,7100  ,0.0       ,USD     ,         ,Questrade Individual margin 10000003; from UCO BUY       ,
UCO     ,2023-01-14,2023-01-15     ,BUY   ,10.0        ,7     ,1.0       ,USD     ,         ,Questrade Individual margin 10000003                     ,
CCO     ,2023-01-16,2023-01-17     ,SELL  ,10.0        ,8     ,1.0       ,CAD     ,(R)      ,Questrade Individual TFSA 10000001                       ,
CCO     ,2023-01-18,2023-01-19     ,SELL  ,10.0        ,9     ,1.0       ,CAD     ,(R)      ,Questrade Individual RRSP 10000002                       ,
CCO     ,2023-01-20,2023-01-21     ,SELL  ,10.0        ,10    ,1.0       ,CAD     ,         ,Questrade Individual margin 10000003                     ,
USD.FX  ,2023-01-22,2023-01-22     ,BUY   ,0.01        ,10900 ,0.0       ,USD     ,(R)      ,Questrade Individual TFSA 10000001; from UCO SELL        ,
UCO     ,2023-01-22,2023-01-23     ,SELL  ,10.0        ,11    ,1.0       ,USD     ,(R)      ,Questrade Individual TFSA 10000001                       ,
USD.FX  ,2023-01-24,2023-01-24     ,BUY   ,0.01        ,11900 ,0.0       ,USD     ,(R)      ,Questrade Individual RRSP 10000002; from UCO SELL        ,
UCO     ,2023-01-24,2023-01-25     ,SELL  ,10.0        ,12    ,1.0       ,USD     ,(R)      ,Questrade Individual RRSP 10000002                       ,
USD.FX  ,2023-01-26,2023-01-26     ,BUY   ,0.01        ,12900 ,0.0       ,USD     ,         ,Questrade Individual margin 10000003; from UCO SELL      ,
UCO     ,2023-01-26,2023-01-27     ,SELL  ,10.0        ,13    ,1.0       ,USD     ,         ,Questrade Individual margin 10000003                     ,
USD.FX  ,2023-01-28,2023-01-28     ,BUY   ,0.01        ,3050  ,0.0       ,USD     ,         ,Questrade Individual margin 10000003; DIV from UCO       ,
UCO     ,2023-01-28,2023-01-29     ,BUY   ,0.0         ,20    ,0.0       ,USD     ,         ,Questrade Individual margin 10000003; From DIS action.   ,
UCO     ,2023-01-28,2023-01-29     ,SELL  ,0.0         ,19    ,0.0       ,USD     ,         ,Questrade Individual margin 10000003; From LIQ action.   ,
"""

   verify_csv_from_text(lines, exp_csv)
   assert errs == []

   def include_lines(matching_pattern):
       lines = [l for l in exp_csv.splitlines()
                if re.search(matching_pattern, l) or 'Security,' in l]
       assert len(lines) > 1 # sanity check
       return '\n'.join(lines)

   def exclude_lines(matching_pattern):
       lines = [l for l in exp_csv.splitlines()
                if not re.search(matching_pattern, l) or 'Security,' in l]
       assert len(lines) > 1 # sanity check
       return '\n'.join(lines)

   # Test filters
   lines, errs = run_and_get_lines(['-a', 'margin'])
   verify_csv_from_text(lines, include_lines('margin'))
   assert errs == []

   lines, errs = run_and_get_lines(['-a', 'margin', '--security', 'UCO'])
   verify_csv_from_text(lines, include_lines('UCO.*margin'))
   assert errs == []

   lines, errs = run_and_get_lines(['-a', '.', '--no-fx'])
   verify_csv_from_text(lines, exclude_lines('USD.FX'))
   assert errs == []

def test_fxt_basic():
   lines, errs = run_and_get_lines(['-a', '.', '--sheet', '2'])

   verify_csv_from_text(lines, """
Security,Trade Date,Settlement Date,Action,Amount/Share,Shares,Commission,Currency,Affiliate,Memo                                     ,Exchange Rate
USD.FX  ,2023-02-05,2023-02-05     ,BUY   ,0.01        ,10000 ,0.0       ,USD     ,         ,Questrade Individual margin 10000003; FXT,1.3
USD.FX  ,2023-02-05,2023-02-05     ,SELL  ,0.01        ,20000 ,0.0       ,USD     ,         ,Questrade Individual margin 10000003; FXT,1.25
""")

   assert errs == []

   # Filter all FXTs
   lines, errs = run_and_get_lines(['-a', '.', '--sheet', '2', '--no-fx'])
   verify_csv(lines, [header])
   assert errs == []

def test_tx_errors():
   lines, errs = run_and_get_lines(['-a', '.', '--sheet', '3'])

   verify_csv(lines, [header])

   assert errs == error_text_to_errors("""
 - Row 2: Could not parse date from '2023-1-7'
 - Row 3: Could not parse date from None
 - Row 4: Could not parse date from '2023-1-8'
 - Row 5: Could not parse date from None
 - Row 6: Unrecognized transaction action 'XXX'
 - Row 7: Symbol was empty
 - Row 8: Unable to parse integer from '2.5' in Quantity column
 - Row 10: Unable to parse number from None in Quantity column
 - Row 11: Unable to parse number from None in Quantity column
 - Row 12: Unable to parse number from 'abc' in Quantity column
 - Row 13: Unable to parse number from 'abc' in Quantity column
 - Row 14: Unable to parse number from None in Price column
 - Row 15: Unable to parse number from None in Price column
 - Row 16: Unable to parse number from 'abc' in Price column
 - Row 17: Unable to parse number from 'abc' in Price column
 - Row 18: Unable to parse number from None in Commission column
 - Row 19: Unable to parse number from None in Commission column
 - Row 20: Unable to parse number from 'abc' in Commission column
 - Row 21: Unable to parse number from 'abc' in Commission column
""")

def test_fxt_errors():
   lines, errs = run_and_get_lines(['-a', '.', '--sheet', '4'])

   verify_csv_from_text(lines, """
Security,Trade Date,Settlement Date,Action,Amount/Share,Shares,Commission,Currency,Affiliate,Memo                              ,Exchange Rate
FOO     ,2023-01-04,2023-01-05     ,BUY   ,10.0        ,2     ,3.99      ,CAD     ,(R)      ,Questrade Individual TFSA 10000001,
""")

   assert errs == error_text_to_errors("""
 - Row 5: Both FXTs have positive amounts
 - Row 7: Both FXTs have negative amounts
 - Row 9: FXTs not supported between CAD and CAD. Exactly one currency must be CAD.
 - Row 11: FXTs not supported between USD and USD. Exactly one currency must be CAD.
 - Row 13: FX currency UNK not supported
 - Row 15: FX currency UNK not supported
 - Row 17: FXTs not supported between UNK and USD. Exactly one currency must be CAD.
 - Row 19: FXTs not supported between USD and UNK. Exactly one currency must be CAD.
 - Row 21: FXTs not supported between UNK2 and UNK1. Exactly one currency must be CAD.
 - Row 22: Unpaired FXT
""")

def test_sort():
   lines, errs = run_and_get_lines(['-a', '.', '--sheet', '5'])

   verify_csv_from_text(lines, """
Security,Trade Date,Settlement Date,Action,Amount/Share,Shares,Commission,Currency,Affiliate,Memo,Exchange Rate
UCO,2023-01-12,2023-01-12,BUY,10.0,1,0.0,USD,,Questrade Individual margin 10000003,
UCO,2023-01-12,2023-01-12,SELL,10.0,3,0.0,USD,,Questrade Individual margin 10000003,
USD.FX,2023-01-12,2023-01-12,BUY,0.01,10000,0.0,USD,,Questrade Individual margin 10000003; FXT,1.3
USD.FX,2023-01-12,2023-01-12,BUY,0.01,3000,0.0,USD,,Questrade Individual margin 10000003; from UCO SELL,
USD.FX,2023-01-12,2023-01-12,BUY,0.01,4000,0.0,USD,,Questrade Individual margin 10000003; from UCO SELL,
USD.FX,2023-01-12,2023-01-12,SELL,0.01,20000,0.0,USD,,Questrade Individual margin 10000003; FXT,1.25
USD.FX,2023-01-12,2023-01-12,SELL,0.01,1000,0.0,USD,,Questrade Individual margin 10000003; from UCO BUY,
USD.FX,2023-01-12,2023-01-12,SELL,0.01,2000,0.0,USD,,Questrade Individual margin 10000003; from UCO BUY,
UCO,2023-01-12,2023-01-13,BUY,10.0,2,0.0,USD,,Questrade Individual margin 10000003,
UCO,2023-01-12,2023-01-13,SELL,10.0,4,0.0,USD,,Questrade Individual margin 10000003,
UCO,2023-01-13,2023-01-13,SELL,10.0,5,0.0,USD,,Questrade Individual margin 10000003,
USD.FX,2023-01-13,2023-01-13,BUY,0.01,5000,0.0,USD,,Questrade Individual margin 10000003; from UCO SELL,
""")
   assert errs == []
