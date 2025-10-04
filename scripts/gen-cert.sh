#!/bin/bash

OUT_DIR="./.cert"
if [ $# -ge 1 ]; then
    OUT_DIR="$1"
fi

mkdir -p "$OUT_DIR"

openssl req -x509 -newkey rsa:2048 -nodes \
    -keyout "$OUT_DIR/key.pem" \
    -out "$OUT_DIR/cert.pem" \
    -subj "/CN=localhost" \
    -days 365 \
    -addext "basicConstraints=CA:FALSE" \
    -addext "subjectAltName=DNS:localhost,DNS:127.0.0.1,IP:127.0.0.1"

echo "Certificate and key generated:"
echo "  Private key: $OUT_DIR/key.pem"
echo "  Certificate: $OUT_DIR/cert.pem"
