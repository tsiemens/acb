---
name: spreadsheets
description: Read, write, and edit CSV and XLSX spreadsheet files. Use when the user asks to view, create, modify, or convert spreadsheet data, or when you need to inspect/produce CSV or XLSX files as part of a task.
argument-hint: [read|write|edit|convert] [file-path] [options...]
allowed-tools: Bash, Read, Write, Glob
---

# Spreadsheets Skill

Helper scripts live in `${CLAUDE_SKILL_DIR}/scripts/`. All scripts are invoked
via the wrapper `${CLAUDE_SKILL_DIR}/scripts/run.sh`, which manages a uv venv
automatically.

## Available commands

### Read a spreadsheet (CSV or XLSX)

```
${CLAUDE_SKILL_DIR}/scripts/run.sh read <file> [--sheet <name-or-index>] [--rows <start>:<end>] [--cols <A:Z or 0:N>]
```

Prints the spreadsheet contents as a formatted table to stdout.
- `--sheet`: For XLSX files, select sheet by name or 0-based index (default: 0).
- `--rows`: Row range (1-based, inclusive). E.g. `--rows 1:10` for first 10 rows.
- `--cols`: Column range by letter (A:C) or 0-based index (0:2).

### Write a new spreadsheet

```
${CLAUDE_SKILL_DIR}/scripts/run.sh write <output-file> [--sheet <name>] <<< '<json-data>'
```

Creates a new CSV or XLSX file. Input is JSON on stdin:
```json
{
  "headers": ["Col1", "Col2"],
  "rows": [["a", "b"], ["c", "d"]]
}
```

For XLSX, `--sheet` sets the sheet name (default: "Sheet1").

### Edit cells in a spreadsheet

```
${CLAUDE_SKILL_DIR}/scripts/run.sh edit <file> [--sheet <name-or-index>] --set <cell>=<value> [--set <cell>=<value> ...]
```

Modifies specific cells in-place. Cell references use A1 notation (e.g. `--set B3=hello`).
For CSV files, the sheet option is ignored.

### Convert between formats

```
${CLAUDE_SKILL_DIR}/scripts/run.sh convert <input-file> <output-file> [--sheet <name-or-index>]
```

Converts between CSV and XLSX (and vice versa).

## Usage guidelines

- For **reading** spreadsheets, prefer the `read` command over `cat` or the Read tool, as it handles XLSX and formats output nicely.
- For **creating test data files** (CSV or XLSX), use the `write` command with JSON input.
- For **small CSV edits**, the Edit tool on the raw file is fine. Use the `edit` command for XLSX files or when you need A1-notation cell addressing.
- The scripts auto-install dependencies (openpyxl) into a local venv on first run.
