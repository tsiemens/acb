"""
Run this file with <repo>/target/{debug|release}/python

This script is used internally by the crate's pdf module as an alternative
PDF reader engine.
"""

import argparse

import pypdf

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('fname')
    ap.add_argument('--show-page-numbers', '-n', action='store_true',
                    help="Show pretty page deliniators")
    ap.add_argument('--parsable-page-markers', '-m', action='store_true',
                    help="Inserts page deliniators that can be parsed back out."
                         "Generally for generating test data.")
    ap.add_argument('--page', '-p', action='append', type=int, dest="pages",
                    help="Print only these pages. Can be provided multiple times.")
    args = ap.parse_args()

    reader = pypdf. PdfReader(args.fname)
    for i, page in enumerate(reader.pages):
        page_num = i + 1
        if args.pages and page_num not in args.pages:
            continue
        text = page.extract_text()
        if args.show_page_numbers:
            if i > 0:
                print()
            print(f"---------- Page {page_num} ----------")
        elif args.parsable_page_markers:
            print(f"PAGE_BREAK<{page_num}>", end='')
        print(text)

if __name__ == '__main__':
    main()