#!/usr/bin/env bash
set -e
if [ -z "$GITLAB_API_PAT" ]; then
  echo "Error: GITLAB_API_PAT environment variable is not set."
  exit 1
fi

PROJECT_ID=80082599
PROJECT_NAME=$(basename "$(git rev-parse --show-toplevel)")
PROJECT_API_URL=https://gitlab.com/api/v4/projects/${PROJECT_ID}
TAG=$(git describe --tags --abbrev=0)

# First run `just build-release` for each target (you'll need zig for that):
#   RUST_TARGET=x86_64-unknown-linux-musl just build-release
#   RUST_TARGET=aarch64-unknown-linux-musl just build-release
#   RUST_TARGET=x86_64-pc-windows-gnu just build-release

upload() {
  local artifact="$1"
  local binary="$2"
  curl --fail \
    --header "PRIVATE-TOKEN: ${GITLAB_API_PAT}" \
    --upload-file "${binary}" \
    "${PROJECT_API_URL}/packages/generic/${PROJECT_NAME}/${TAG}/${artifact}"
}

upload "${PROJECT_NAME}-linux-amd64"       "target/x86_64-unknown-linux-musl/release/${PROJECT_NAME}"
upload "${PROJECT_NAME}-linux-arm64"       "target/aarch64-unknown-linux-musl/release/${PROJECT_NAME}"
upload "${PROJECT_NAME}-windows-amd64.exe" "target/x86_64-pc-windows-gnu/release/${PROJECT_NAME}.exe"
