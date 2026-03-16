#!/usr/bin/env bash
set -e

PROJECT_ID=80082599
PROJECT_NAME=hermit
PROJECT_API_URL=https://gitlab.com/api/v4/projects/${PROJECT_ID}
PROJECT_TARGET_DIR=/usr/local/bin/${PROJECT_NAME}

case "$(uname -m)" in
  x86_64)  ARTIFACT_NAME="hermit-linux-amd64" ;;
  aarch64) ARTIFACT_NAME="hermit-linux-arm64" ;;
  *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac

TAG=$(curl -fsSL "${PROJECT_API_URL}/releases/permalink/latest" | grep -o '"tag_name":"[^"]*"' | cut -d'"' -f4)

sudo curl -fsSL "${PROJECT_API_URL}/packages/generic/${PROJECT_NAME}/${TAG}/${ARTIFACT_NAME}" -o $PROJECT_TARGET_DIR

sudo chmod +x $PROJECT_TARGET_DIR

echo "${PROJECT_NAME} installed to ${PROJECT_TARGET_DIR} (tag: ${TAG}, artifact name: ${ARTIFACT_NAME})"
