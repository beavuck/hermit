#!/usr/bin/env bash
set -e
if [ -z "$GITLAB_API_PAT" ]; then
  echo "Error: GITLAB_API_PAT environment variable is not set."
  exit 1
fi

#vvvvvvvv CHANGE AT NEED vvvvvvvv#

TAG=1.2.0
BINARY=~/Documents/dev/hermit/target/x86_64-unknown-linux-musl/release/hermit

#^^^^^^^^ CHANGE AT NEED ^^^^^^^^^#

PROJECT_ID=80082599
PROJECT_NAME=hermit
ARTIFACT_NAME=hermit
PROJECT_API_URL=https://gitlab.com/api/v4/projects/${PROJECT_ID}

# First run `just build-release`

curl --fail \
  --header "PRIVATE-TOKEN: ${GITLAB_API_PAT}" \
  --upload-file "${BINARY}" \
  "${PROJECT_API_URL}/packages/generic/${PROJECT_NAME}/${TAG}/${ARTIFACT_NAME}"
