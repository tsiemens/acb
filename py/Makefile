setup:
	test -d .venv || ./setup.sh

update:
	.venv/bin/pip install -r requirements.txt

test:
	./.run-in-venv.sh pytest ../pytest
