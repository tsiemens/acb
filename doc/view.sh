#!/usr/bin/env bash
SCRIPTPATH="$( cd "$(dirname "$0")" ; pwd -P )"
groff -man -Tascii $SCRIPTPATH/acb.1
