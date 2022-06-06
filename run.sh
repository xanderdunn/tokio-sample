#!/usr/bin/env bash

set -e

filename="target/x86_64-unknown-linux-gnu/debug/tokio_demo"

sudo chown -R $USER target
echo "Deleting $filename"
rm -f "$filename"
docker compose -f docker/docker-compose-run.yml --project-directory ./ up --abort-on-container-exit --build
