#!/usr/bin/env bash

# A manual test script for sanitizing ETRADE PDF files.
# Extracts text from a PDF and sanitizes it according to the rules defined in
# py/sanitize_config.toml, then runs a diff against the un-sanitized output
# so the user can verify that no personal information is present.
#
# Should be run from the root of the prpoject directory.

DIFFER="code --diff"
PDF="$1"
UNCONVERTED_TEXT_FILE=/tmp/sanitize_etrade_test_pdf_test_baseline.txt
CONVERTED_TEXT_FILE=/tmp/sanitize_etrade_test_pdf_test_sanitized.txt
target/debug/pdf-text "$PDF" > $UNCONVERTED_TEXT_FILE &&
   cat $UNCONVERTED_TEXT_FILE | py/sanitize-etrade-test-pdf.py > $CONVERTED_TEXT_FILE &&
   $DIFFER $UNCONVERTED_TEXT_FILE $CONVERTED_TEXT_FILE
