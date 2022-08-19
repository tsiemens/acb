"""
Checks if the script is running in a virtualenv.
If it is not, activates the venv and restarts the script.

Simply import this module before any third-party pip packages in an
executable script, and it should automatically run under the venv.

This replaces the need for additional shell script wrappers or manually
sourcing the activation script.
"""
import sys

in_venv = False
for p in sys.path:
   if '.venv/lib/' in p:
      in_venv = True
      break

if not in_venv:
   # import here for efficiency
   import os
   dirname = os.path.dirname(__file__)
   script = f'cd {dirname}; source {dirname}/.venv/bin/activate; cd - >/dev/null; \
exec python3 {sys.argv[0]} "$@"'
   os.execvp('bash', ['bash' ,'-c', script, 'bash'] + sys.argv[1:])
