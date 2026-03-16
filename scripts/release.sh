#!/usr/bin/env bash
set -e
if [ -z "$GITLAB_API_PAT" ]; then
  echo "Error: GITLAB_API_PAT environment variable is not set."
  exit 1
fi

PROJECT_ID=80082599
PROJECT_NAME=$(basename "$(git rev-parse --show-toplevel)")
PROJECT_API_URL=https://gitlab.com/api/v4/projects/${PROJECT_ID}
PROJECT_URL=https://gitlab.com/beavuck-services/${PROJECT_NAME}
TAG=$(git describe --tags --abbrev=0)

# First run publish.sh

curl --fail \
  --header "PRIVATE-TOKEN: $GITLAB_API_PAT" \
  --header "Content-Type: application/json" \
  --data "{
    \"name\": \"${TAG}\",
    \"tag_name\": \"${TAG}\",
    \"description\": \"Details: ${PROJECT_URL}/-/network/main?ref_type=heads\",
    \"assets\": {
      \"links\": [
        {
          \"name\": \"linux-amd64\",
          \"url\": \"${PROJECT_API_URL}/packages/generic/${PROJECT_NAME}/${TAG}/${PROJECT_NAME}-linux-amd64\",
          \"link_type\": \"package\"
        },
        {
          \"name\": \"linux-arm64\",
          \"url\": \"${PROJECT_API_URL}/packages/generic/${PROJECT_NAME}/${TAG}/${PROJECT_NAME}-linux-arm64\",
          \"link_type\": \"package\"
        },
        {
          \"name\": \"windows-amd64\",
          \"url\": \"${PROJECT_API_URL}/packages/generic/${PROJECT_NAME}/${TAG}/${PROJECT_NAME}-windows-amd64.exe\",
          \"link_type\": \"package\"
        }
      ]
    }
  }" \
  "${PROJECT_API_URL}/releases"
