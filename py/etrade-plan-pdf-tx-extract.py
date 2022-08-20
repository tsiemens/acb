#!/usr/bin/env python3

"""
A convenience script to extract transactions from PDFs downloaded from us.etrade.com
"""
import ensure_in_venv

import argparse
from contextlib import contextmanager
from dataclasses import dataclass
import datetime
from decimal import Decimal
import io
import itertools
from pprint import pprint
import re
import sys
from typing import Union

import PyPDF2

@dataclass
class BenefitEntry:
   security: str

   acquire_tx_date: datetime.date
   acquire_settle_date: datetime.date
   acquire_share_price: Decimal
   acquire_shares: int

   sell_to_cover_tx_date: datetime.date
   sell_to_cover_settle_date: datetime.date
   sell_to_cover_price: Decimal
   sell_to_cover_shares: int
   sell_to_cover_fee: Decimal

   plan_note: str

def text_to_common_data(text: str) -> dict:
   return {
      "employee id": re.search(r"Employee ID:\s*(\d+)", text).group(1),
      "account number": re.search(r"Account Number\s*(\d+)", text).group(1),
      "symbol": re.search(r"Company Name\s*\(Symbol\)*.*\(([A-Za-z\.]+)\)", text, re.S).group(1),
   }

def text_to_rsu_data(text: str) -> dict:
   return text_to_common_data(text) | {
      "release date": datetime.datetime.strptime(
         re.search(r"Release Date\s*(\d+-\d+-\d+)", text).group(1), "%m-%d-%Y"
      ).date(),
      "award number": re.search(r"Award Number\s*(R\d+)", text).group(1),
      "share released": Decimal(
         re.search(r"Shares Released\s*(\d+\.\d+)", text).group(1)
      ),
      "share sold": Decimal(
         re.search(r"Shares Sold\s*\((\d+\.\d+)\)", text).group(1)
      ),
      "share issued": Decimal(
         re.search(r"Shares Issued\s*(\d+\.\d+)", text).group(1),
      ),
      "FMV": Decimal(
         re.search(r"Market Value Per Share\s*\$(\d+\.\d+)", text).group(1)
      ),
      "sale price": Decimal(
         re.search(r"Sale Price Per Share\s*\$(\d+\.\d+)", text).group(1)
      ),
      "market value": Decimal(
         re.search(r"Market Value\s*\$([\d,]+\.\d+)", text)
         .group(1)
         .replace(',', '')
      ),
      "total sale price": Decimal(
         re.search(r"Total Sale Price\s*\$([\d,]+\.\d+)", text)
         .group(1)
         .replace(',', '')
      ),
      "total tax": Decimal(
         re.search(r"Total Tax\s*\$([\d,]+\.\d+)", text)
         .group(1)
         .replace(',', '')
      ),
      "fee": Decimal(re.search(r"Fee\s*\(\$(\d+\.\d+)", text).group(1)),
      "cash leftover": Decimal(
         re.search(r"Total Due Participant\s*\$(\d+\.\d+)", text).group(1)
      ),
   }

def text_to_rsu_entry(text: str) -> BenefitEntry:
   data = text_to_rsu_data(text)
   return BenefitEntry(
         security=data['symbol'],

         # The FMV is for the release date, so treat that as the tx date.
         acquire_tx_date=data['release date'],
         # There is no way to know the settlement date in RSU distributions.
         # Since they are never near the year-end boundary, just use the release date.
         acquire_settle_date=data['release date'],
         acquire_share_price=data['FMV'],
         acquire_shares=int(data['share released']),

         # The sell-to-cover date is almost always a day or two after the release
         # date. This needs to be looked up separately if we want an accurate
         # USD/CAD exchange rate.
         sell_to_cover_tx_date=None,
         sell_to_cover_settle_date=None,
         sell_to_cover_price=data['sale price'],
         sell_to_cover_shares=int(data['share sold']),
         sell_to_cover_fee=data['fee'],

         plan_note="RSU " + data['award number'],
      )

def text_to_espp_data(text) -> dict:
   return text_to_common_data(text) | {
      "purchase date": datetime.datetime.strptime(
         re.search(r"Purchase Date\s*(\d+-\d+-\d+)", text).group(1), "%m-%d-%Y"
      ).date(),
      "share purchased": Decimal(
         re.search(r"Shares Purchased\s*(\d+\.\d+)", text).group(1)
      ),
      "share sold": Decimal(
         re.search(r"Shares Sold to Cover Taxes\s*(\d+\.\d+)", text).group(1)
      ),
      "FMV": Decimal(
         re.search(r"Purchase Value per Share\s*\$(\d+\.\d+)", text).group(1)
      ),
      "purchase price": Decimal(
         re.search(
            r"Purchase Price per Share\s*\([^\)]*\)\s*\$(\d+\.\d+)", text, re.S
         ).group(1)
      ),
      "total price": Decimal(
         re.search(r"Total Price\s*\(\$([\d,]+\.\d+)\)", text)
         .group(1)
         .replace(',', '')
      ),
      "total value": Decimal(
         re.search(r"Total Value\s*\$([\d,]+\.\d+)", text)
         .group(1)
         .replace(',', '')
      ),
      "taxable gain": Decimal(
         re.search(r"Taxable Gain\s*\$([\d,]+\.\d+)", text)
         .group(1)
         .replace(',', '')
      ),
      "sale price": Decimal(
         re.search(
            r"Sale Price for Shares Sold to Cover Taxes\s*\$(\d+\.\d+)", text
         ).group(1)
      ),
      "fee": Decimal(re.search(r"Fees\s*\(\$(\d+\.\d+)", text).group(1)),
      "market value at grant": Decimal(
         re.search(r"Market Value\s*\$([\d,]+\.\d+)", text)
         .group(1)
         .replace(',', '')
      ),
      "total sale price": Decimal(
         re.search(r"Value Of Shares Sold\s\$([\d,]+\.\d+)", text)
         .group(1)
         .replace(',', '')
      ),
      "cash leftover": Decimal(
         re.search(r"Amount in Excess of Tax Due\s\$(\d+\.\d+)", text).group(
            1
         )
      ),
      "total tax": Decimal(
         re.search(
            r"Total Taxes Collected at purchase\s\(\$([\d,]+\.\d+)\)", text
         )
         .group(1)
         .replace(',', '')
      ),
   }

def text_to_espp_entry(text: str) -> BenefitEntry:
   data = text_to_espp_data(text)
   return BenefitEntry(
         security=data['symbol'],

         acquire_tx_date=data['purchase date'],
         # There is no way to know the settlement date in ESPP distributions.
         # Since they are never near the year-end boundary, just use the purchase date.
         acquire_settle_date=data['purchase date'],
         acquire_share_price=data['FMV'],
         acquire_shares=int(data['share purchased']),

         # The sell-to-cover date is almost always a day or two after the release
         # date. This needs to be looked up separately if we want an accurate
         # USD/CAD exchange rate.
         sell_to_cover_tx_date=None,
         sell_to_cover_settle_date=None,
         sell_to_cover_price=data['sale price'],
         sell_to_cover_shares=int(data['share sold']),
         sell_to_cover_fee=data['fee'],

         plan_note="ESPP",
      )

@dataclass
class TradeConfirmation:
   security: str
   action: str
   tx_date: datetime.date
   settle_date: datetime.date
   shares: int

def text_to_trade_confirmation_objs(text: str):
   ms = re.finditer(r'(?P<txdate>\d+/\d+/\d+)\s+(?P<sdate>\d+/\d+/\d+)\s+(?P<cpt>\d+)\s+'
                    r'(?P<sym>\S+)\s+(?P<act>\S+)\s+(?P<nshares>\d+)\s+\$(\d+\.\d+)',
                    text)
   objs = []
   for m in ms:
      objs.append(TradeConfirmation(
            security=m.group('sym'),
            action=m.group('act'),
            tx_date=datetime.datetime.strptime(m.group('txdate'), '%m/%d/%y').date(),
            settle_date=datetime.datetime.strptime(m.group('sdate'), '%m/%d/%y').date(),
            shares=int(m.group('nshares')),
         ))
   return objs

@contextmanager
def StderrSilencer():
   with open('/dev/null', 'w+') as devnull:
      stderr = sys.stderr
      sys.stderr = devnull
      yield
      sys.stderr = stderr

def parse_pdf(f: io.BufferedReader) -> Union[BenefitEntry, TradeConfirmation]:
   reader = PyPDF2.PdfReader(f)
   with StderrSilencer():
      text = reader.getPage(0).extractText()

   if re.search(r'Plan\s*ESP2', text):
      obj = text_to_espp_entry(text)
   elif re.search(r'STOCK\s+PLAN\s+RELEASE\s+CONFIRMATION', text):
      obj = text_to_rsu_entry(text)
   elif re.search(r'TRADE\s*CONFIRMATION', text):
      obj = text_to_trade_confirmation_objs(text)
   else:
      pprint(text)
      print("Error: Unrecognized PDF format")
      exit(1)

   pprint(obj)
   return obj

def find_and_apply_sell_to_cover_trade_set(benefit, trade_confs):
   matching_trades = None
   for n in range(len(trade_confs), 0, -1):
      for trades in itertools.combinations(trade_confs, n):
         if not all(t.security == benefit.security for t in trades):
            continue
         n_shares = sum(t.shares for t in trades)
         if n_shares == benefit.sell_to_cover_shares:
            if matching_trades is not None:
               print(f"Error: Multiple trade combinations near {benefit.acquire_tx_date} "
                      "could potentially constitute the sale")
               return []
            else:
               matching_trades = trades

   if matching_trades:
      benefit.sell_to_cover_tx_date=matching_trades[0].tx_date
      benefit.sell_to_cover_settle_date=matching_trades[0].settle_date
      return matching_trades
   else:
      print(f"Error: Found no trades matching the sell-to-cover for {benefit.plan_note} "
            f"{benefit.acquire_tx_date}")
      return []

def amend_benefit_sales(benefits, trade_confs):
   trade_confs = list(trade_confs) # Make a a copy
   for benefit in benefits:
      # Find the sale(s) which could constitute this sell-to-cover
      latest_day = benefit.acquire_tx_date + datetime.timedelta(days=5)
      candidate_trades = []
      for trade in trade_confs:
         if trade.action == 'SELL' and \
            benefit.acquire_tx_date <= trade.tx_date and trade.tx_date <= latest_day:
            candidate_trades.append(trade)

      matched_trades = find_and_apply_sell_to_cover_trade_set(benefit, candidate_trades)
      for t in matched_trades:
         trade_confs.remove(t)

   # Return leftover trades
   return trade_confs

def main():
   ap = argparse.ArgumentParser(description="""\
Instructions:
Go to us.etrade.com, log into your account, and go to 'Stock Plan', then to
'Holdings'. In ESPP and RS sections, click 'Benefit History'. Expand each relevant
section, and donwload (right-click and 'save link as') each
'View confirmation of purchase' or 'View confirmation of release' link PDF.

Then go to 'Account', then 'Documents' > 'Trade Confirmations.' Adjust the date
range, and download the trade confirmation PDF for each sale.
Note: For sales on the same day, both appear on the same PDF. The download link
for both sales is to the same document, so only one needs to be downloaded.

Run this script, giving the name of all PDFs as arguments.""",
         formatter_class=argparse.RawDescriptionHelpFormatter,)
   ap.add_argument('files', metavar='FILES', nargs='+')
   args = ap.parse_args()

   benefits = []
   trade_confs = []
   first = True
   for fname in args.files:
      if not first:
         print()
      first = False
      print("Parsing ", fname)
      with open(fname, 'rb') as f:
         obj = parse_pdf(f)
         if isinstance(obj, BenefitEntry):
            benefits.append(obj)
         elif isinstance(obj, list) and obj and isinstance(obj[0], TradeConfirmation):
            trade_confs.extend(obj)

   remaining_trades = amend_benefit_sales(benefits, trade_confs)
   print("\nAmmended benefit entries:")
   for b in benefits:
      pprint(b)
   print("\nRemaining trades:")
   for t in remaining_trades:
      pprint(t)

if __name__ == '__main__':
   exit(main())

