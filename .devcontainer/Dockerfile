FROM mcr.microsoft.com/devcontainers/rust:latest

RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    python3-venv

RUN pip install pre-commit --break-system-packages
