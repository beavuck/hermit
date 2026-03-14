#!/usr/bin/env bash
set -e

PROJECT_ID=80082599
PROJECT_NAME=hermit
ARTIFACT_NAME=hermit
PROJECT_API_URL=https://gitlab.com/api/v4/projects/${PROJECT_ID}
PROJECT_TARGET_DIR=/usr/local/bin/${PROJECT_NAME}

TAG=$(curl -fsSL "${PROJECT_API_URL}/releases/permalink/latest" | grep -o '"tag_name":"[^"]*"' | cut -d'"' -f4)

sudo curl -fsSL "${PROJECT_API_URL}/packages/generic/${PROJECT_NAME}/${TAG}/${ARTIFACT_NAME}" -o $PROJECT_TARGET_DIR

sudo chmod +x $PROJECT_TARGET_DIR

echo "${PROJECT_NAME} installed to ${PROJECT_TARGET_DIR} (tag: ${TAG}, artifact name: ${ARTIFACT_NAME})"
