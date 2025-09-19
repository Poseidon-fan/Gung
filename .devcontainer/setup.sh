#!/bin/bash

git config --global --add safe.directory $(pwd)

# Install pre-commit hooks
pre-commit install

# Create python venv
python3 -m venv .venv
. .venv/bin/activate

# Install python dependencies
pip install maturin
