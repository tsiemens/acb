#!/usr/bin/env python3
"""Generate dist/samples/manifest.json for the www debug panel.

Walks dist/samples/ (following the debug/ symlink into tests/data) and creates
a manifest listing sample sets grouped as follows:
  - Files at the dist/samples/ root → a single "default" set.
  - Each file directly under dist/samples/debug/ → its own set.
  - Each subdirectory under dist/samples/debug/ that has eligible files
    directly in it → one set per directory.

Exclude patterns filter out test-only files (expected outputs, incompatible
formats, etc.).
"""

import fnmatch
import json
import os
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
SAMPLES_DIR = SCRIPT_DIR.parent / "dist" / "samples"

# Files (and directories) to exclude by exact name or glob pattern.
EXCLUDE_PATTERNS: list[str] = [
    "expected_output.csv",
    "QT_Test_Export.xlsx",
    "manifest.json",
    "*.test",   # e.g. pdfs.test test-helper dirs
]


def is_excluded(name: str) -> bool:
    return any(fnmatch.fnmatch(name, pat) for pat in EXCLUDE_PATTERNS)


def make_set(set_id: str, files: list[dict[str, str]]) -> dict:
    return {"id": set_id, "label": set_id, "files": files}


def main() -> None:
    if not SAMPLES_DIR.exists():
        print(f"ERROR: {SAMPLES_DIR} does not exist.")
        print("Run 'make static-dist-link' (or 'make debug-samples') first.")
        raise SystemExit(1)

    sets: list[dict] = []

    # os.walk with followlinks=True so we descend into the debug/ symlink.
    for dirpath, dirnames, filenames in os.walk(SAMPLES_DIR, followlinks=True):
        # Prune excluded directories in-place to prevent descent.
        dirnames[:] = sorted(d for d in dirnames if not is_excluded(d))

        filenames_ok = sorted(f for f in filenames if not is_excluded(f))
        if not filenames_ok:
            continue

        dir_path = Path(dirpath)
        rel = dir_path.relative_to(SAMPLES_DIR)
        parts = rel.parts  # () for root, ('debug',) for debug/, etc.

        if not parts:
            # Root of dist/samples/ — all files form the "default" set.
            sets.append(make_set("default", [
                {"path": f, "name": f} for f in filenames_ok
            ]))

        elif parts == ("debug",):
            # Files directly in debug/ (= tests/data root): one set per file.
            for f in filenames_ok:
                file_path = f"debug/{f}"
                sets.append(make_set(file_path, [
                    {"path": file_path, "name": f}
                ]))

        else:
            # Subdirectory under debug/: all direct files form one set.
            set_id = str(rel).replace("\\", "/")
            sets.append(make_set(set_id, [
                {"path": f"{set_id}/{f}", "name": f} for f in filenames_ok
            ]))

    manifest_path = SAMPLES_DIR / "manifest.json"
    with open(manifest_path, "w") as fh:
        json.dump({"sets": sets}, fh, indent=2)
    print(f"  manifest.json written ({len(sets)} sets)")


if __name__ == "__main__":
    main()
