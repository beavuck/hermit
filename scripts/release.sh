#!/usr/bin/env bash
set -e
if [ -z "$GITLAB_API_PAT" ]; then
  echo "Error: GITLAB_API_PAT environment variable is not set."
  exit 1
fi

#vvvvvvvv CHANGE AT NEED vvvvvvvv#

TAG=1.2.0

#^^^^^^^^ CHANGE AT NEED ^^^^^^^^^#

PROJECT_ID=80082599
PROJECT_NAME=hermit
ARTIFACT_NAME=hermit
PROJECT_API_URL=https://gitlab.com/api/v4/projects/${PROJECT_ID}
PROJECT_URL=https://gitlab.com/beavuck-services/hermit

# First run publish.sh

curl --fail \
  --header "PRIVATE-TOKEN: $GITLAB_API_PAT" \
  --header "Content-Type: application/json" \
  --data "{
    \"name\": \"${TAG}\",
    \"tag_name\": \"${TAG}\",
    \"description\": \"Details: ${PROJECT_URL}/-/network/main?ref_type=heads\",
    \"assets\": {
      \"links\": [{
        \"name\": \"linux-executable\",
        \"url\": \"${PROJECT_API_URL}/packages/generic/${PROJECT_NAME}/${TAG}/${ARTIFACT_NAME}\",
        \"link_type\": \"package\"
      }]
    }
  }" \
  "${PROJECT_API_URL}/releases"
