#!/usr/bin/env python3

"""
This test file tests the etrade-plan-pdf-tx-extract script.

Because it is difficult to generate accurate pdfs to test against that
are also anonymized, the test data is not checked into the git repo.

Rather, it is either local or in a mounted cloud storage.

For example work-in-progress PDFs may be placed in this test directory under
   test_etrade_plan_pdf_tx_extract.wip.test/
A mounted cloud storage folder may be symlinked to
   test_etrade_plan_pdf_tx_extract.test/

Each of these should be created, and at least one must have a test directory
in it (otherwise warnings/errors are produced to avoid false passes).

Under one or both of these dirs, should be a directory for each test case.
Each test case should simply be a set of PDFs that will be given to the
script, and an expected_output.csv file.

These test directories can be named ending with .disabled to turn them off.

If errors are expected, add an "expected_error.txt" file with text expected
to be output to stderr.
"""

import os
import subprocess
import warnings

import pytest

script_path = os.path.join(os.path.dirname(__file__), '../py/etrade-plan-pdf-tx-extract')

base_dirs = [
   'test_etrade_plan_pdf_tx_extract.test',
   'test_etrade_plan_pdf_tx_extract.wip.test',
]

def base_dir_full_path(p):
   return os.path.join(os.path.dirname(__file__), p)

def get_base_dirs(existing=None):
   if existing is None:
      return base_dirs
   return [d for d in base_dirs if os.path.isdir(base_dir_full_path(d)) == existing]

def get_test_dirs():
   dir_paths = []
   disabled_test_dirs = []
   existing_base_dirs = get_base_dirs(existing=True)
   for d in existing_base_dirs:
      full_base_dir_path = base_dir_full_path(d)
      for test_dir in os.listdir(full_base_dir_path):
         full_test_dir_path = os.path.join(full_base_dir_path, test_dir)
         pretty_test_dir_path = os.path.join(d, test_dir)
         if os.path.isdir(full_test_dir_path):
            if full_test_dir_path.endswith('.disabled'):
               disabled_test_dirs.append(pretty_test_dir_path)
            else:
               dir_paths.append(pretty_test_dir_path)

   if not existing_base_dirs:
      warnings.warn(f"Found no test directories, looked for {base_dirs}")
   elif not dir_paths:
      warnings.warn(f"Found test directories {existing_base_dirs}, but no test case sub-directories")
   if disabled_test_dirs:
      warnings.warn(f"Found disabled test directories {disabled_test_dirs}")

   return dir_paths

def test_dir_presence():
   """Sanity check (warn) if the expected test directories have not been
   created, since they are local to the machine.
   """
   assert get_base_dirs(existing=True), "No test dirs found to run against"

def run(options: list[str]) -> subprocess.CompletedProcess:
   cmd = [script_path] + options
   env = dict(os.environ)
   env["PYTHONWARNINGS"] = "ignore:::PyPDF2._cmap"
   return subprocess.run(cmd, capture_output=True, env=env)

def unify_newlines(text):
   return text.replace('\r', '')

@pytest.mark.parametrize("test_dir", get_test_dirs())
def test_script(test_dir):
   full_test_dir_path = base_dir_full_path(test_dir)
   pdfs = [os.path.join(full_test_dir_path, f) for f in os.listdir(full_test_dir_path) if f.endswith('.pdf')]
   assert pdfs, f"No pdfs found in {full_test_dir_path}"

   exp_output = None
   output_path = os.path.join(full_test_dir_path, 'expected_output.csv')
   if os.path.exists(output_path):
      with open(output_path) as f:
         exp_output = f.read()

   exp_err = ""
   err_path = os.path.join(full_test_dir_path, 'expected_error.txt')
   if os.path.exists(err_path):
      with open(err_path) as f:
         exp_err = f.read()

   ret = run(pdfs)
   if exp_output is not None or exp_err is None:
      assert unify_newlines(ret.stdout.decode()) == unify_newlines(exp_output)

   assert unify_newlines(ret.stderr.decode()) == unify_newlines(exp_err)
