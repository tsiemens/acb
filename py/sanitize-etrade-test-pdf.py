#!/usr/bin/env python3

import argparse
from datetime import datetime
import hashlib
import os
import re
import sys
import toml

DESCRIPTION = """
Sanitize E*TRADE PDF text for public test use.

This script reads a TOML configuration file to define regex replacements
for sensitive information in E*TRADE PDF text files. It (deterministically) randomizes
dollar and share values, to simplify the re-generation of the test data from the
original PDFs.

Usage:
    pdf-text <input_file> | sanitize-etrade-test-pdf.py
"""

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    default_config = os.path.join(script_dir, 'sanitize_config.toml')

    parser = argparse.ArgumentParser(description='Sanitize E*TRADE PDF text for public test use.')
    parser.add_argument('--config', type=str, default=default_config, help='Path to the TOML config file')
    args = parser.parse_args()

    config = toml.load(args.config)
    replacements = config.get('replacements', [])
    randomizer = config.get('randomizer', {})

    # Read input text
    input_text = sys.stdin.read()

    # Apply regex replacements (including addresses)
    for rep in replacements:
        pattern = rep['pattern']
        repl = rep['replacement']
        input_text = re.sub(pattern, repl, input_text, flags=re.IGNORECASE)

    # Hardcoded grant number (R number) transformation
    def grant_number_repl(match):
        # Optionally, shuffle or mask the numbers more in the future
        return f"R123{match.group(1)}{match.group(2)}"

    input_text = re.sub(r'R[0-9]([0-9])[0-9]+([0-9])\b', grant_number_repl, input_text)

    # Find all dates in the text
    DATE_REGEX = r'\b\d{4}-\d{2}-\d{2}\b'
    dates = re.findall(DATE_REGEX, input_text)
    if dates:
        most_recent_date = max(dates, key=lambda d: datetime.strptime(d, '%Y-%m-%d'))
        year_month = most_recent_date[:7]  # YYYY-MM
    else:
        year_month = 'default'

    # Seeds for randomizer (allow separate for shares and dollars)
    dollar_seed = randomizer.get('dollar_seed', randomizer.get('seed', 'default')) + year_month
    share_seed = randomizer.get('share_seed', randomizer.get('seed', 'default')) + year_month
    dollar_hash = int(hashlib.sha256(dollar_seed.encode()).hexdigest(), 16)
    share_hash = int(hashlib.sha256(share_seed.encode()).hexdigest(), 16)
    dollar_multiplier = 0.2 + 1.6 * ((dollar_hash % 1000) / 1000.0)  # Example: 0.2-1.8
    share_multiplier = 0.2 + 1.6 * ((share_hash % 1000) / 1000.0)  # Example: 0.2-1.8

    # Randomize dollar/share values
    DOLLAR_THRESHOLD = randomizer.get('dollar_threshold', 30.0)
    SHARE_THRESHOLD = randomizer.get('share_threshold', 5.0)
    # Match optional $ (with optional whitespace), then floats with optional comma separators
    FLOAT_REGEX = r'(\$\s*)?(\d{1,3}(?:,\d{3})*|\d+)\.(\d+)'

    def randomize_value(match):
        is_dollar = match.group(1) is not None
        int_part = match.group(2)
        dec_part = match.group(3)
        num_str = f"{int_part}.{dec_part}"
        # Remove commas for float conversion
        val = float(num_str.replace(',', ''))
        if is_dollar:
            if val >= DOLLAR_THRESHOLD:
                val = round(val * dollar_multiplier, 2)
        else:
            if val >= SHARE_THRESHOLD:
                # For shares, check if original was whole number
                is_whole = float(dec_part) == 0
                val = val * share_multiplier
                if is_whole:
                    val = round(val)  # Round to whole number if original was whole
                else:
                    val = round(val, 4)  # Keep 4 decimals if original was fractional
        # Format with commas if original had commas
        if ',' in int_part:
            int_fmt = f"{int(val):,}"
        else:
            int_fmt = str(int(val))
        # Preserve the original number of decimals
        decimals = len(dec_part)
        val_str = f"{val:.{decimals}f}"
        # Format integer part with commas if original had commas
        int_val = int(float(val_str))
        if ',' in int_part:
            int_fmt = f"{int_val:,}"
        else:
            int_fmt = str(int_val)
        result = f'{int_fmt}.{val_str.split(".")[1]}'
        if is_dollar:
            return f'{match.group(1) or "$"}{result}'
        else:
            return result

    input_text = re.sub(FLOAT_REGEX, randomize_value, input_text)

    # Output sanitized text
    print(input_text, end='')

if __name__ == '__main__':
    main()
