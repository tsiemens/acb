#!/usr/bin/env bash
DIR=$(dirname `realpath $0`)
set +x
cd $DIR
python3 -m venv .venv
.venv/bin/pip install -r requirements.txt
