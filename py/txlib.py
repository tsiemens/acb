from contextlib import contextmanager
import csv
from dataclasses import dataclass
import datetime
from enum import Enum
import sys

# Output format:
# Security,Date,Action,Amount/Share,Shares,Commission,Currency,Memo,Exchange Rate
# SPXT,2021-01-19,Buy,65.9,150,0,USD,Settlement date. Trade date:14-01-21,

class Action(Enum):
   BUY = 'BUY'
   SELL = 'SELL'

@dataclass
class Tx:
   security: str
   trade_date: str
   settlement_date: str # Settlement date
   date_and_time: str # Just used for sorting
   action: Action
   amount_per_share: float
   num_shares: int
   commission: float
   currency: str
   memo: str
   exchange_rate: float
   affiliate: str

   # optional metadata
   account: str = None

   @staticmethod
   def date_to_str(d: datetime.date) -> str:
      """Returns a yyyy-mm-dd formatted date string"""
      return f"{d.year}-{d.month:02}-{d.day:02}"

class AcbCsvRenderer:
   def __init__(self):
      self.txs = []
      self.errors = []

   def sort_txs(self):
      self.txs = sorted(self.txs, key=lambda t: t.date_and_time)

   def rows(self):
      for tx in self.txs:
         yield (tx.security, tx.trade_date, tx.settlement_date, tx.action.name,
                str(tx.amount_per_share),
                str(tx.num_shares), str(tx.commission), tx.currency,
                tx.affiliate, tx.memo,
                str(tx.exchange_rate) if tx.exchange_rate else '')

   def header_row(self):
      return ['Security', 'Trade Date', 'Settlement Date', 'Action', 'Amount/Share',
              'Shares', 'Commission', 'Currency', 'Affiliate', 'Memo', 'Exchange Rate']

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
         assert len(row) == len(col_widths)
         for i, col in enumerate(row):
            col_widths[i] = max(col_widths[i], len(str(col)))

      fmt = '|'.join('{:%d}' % w for w in col_widths)
      for row in all_rows:
         row_strs = (str(v) for v in row)
         print(fmt.format(*row_strs))

   def render_errors(self) -> bool:
      """
      Returns True if there were errors
      """
      if self.errors:
         print("\nErrors:", file=sys.stderr)
         for err in self.errors:
            print(" -", err, file=sys.stderr)
         return True
      return False

@contextmanager
def StderrSilencer():
   with open('/dev/null', 'w+') as devnull:
      stderr = sys.stderr
      sys.stderr = devnull
      yield
      sys.stderr = stderr
