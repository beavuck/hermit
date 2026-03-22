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
PREV_TAG=$(git describe --tags --abbrev=0 "${TAG}^")
RELEASE_DESCRIPTION=$(bash ./list_changes.sh "$PREV_TAG" "$TAG")

# First run publish.sh

PAYLOAD=$(jq -n \
  --arg name "$TAG" \
  --arg tag_name "$TAG" \
  --arg description "$RELEASE_DESCRIPTION" \
  --arg linux_amd64 "${PROJECT_API_URL}/packages/generic/${PROJECT_NAME}/${TAG}/${PROJECT_NAME}-linux-amd64" \
  --arg linux_arm64 "${PROJECT_API_URL}/packages/generic/${PROJECT_NAME}/${TAG}/${PROJECT_NAME}-linux-arm64" \
  --arg windows_amd64 "${PROJECT_API_URL}/packages/generic/${PROJECT_NAME}/${TAG}/${PROJECT_NAME}-windows-amd64.exe" \
  '{name: $name, tag_name: $tag_name, description: $description, assets: {links: [
    {name: "linux-amd64",    url: $linux_amd64,    link_type: "package"},
    {name: "linux-arm64",    url: $linux_arm64,    link_type: "package"},
    {name: "windows-amd64",  url: $windows_amd64,  link_type: "package"}
  ]}}')

curl --fail \
  --header "PRIVATE-TOKEN: $GITLAB_API_PAT" \
  --header "Content-Type: application/json" \
  --data "$PAYLOAD" \
  "${PROJECT_API_URL}/releases"
