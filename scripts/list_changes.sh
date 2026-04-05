#!/usr/bin/env bash

set -euo pipefail

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  echo "Usage: $0 [<from> [<to>]]"
  echo "  <from> and <to> can be tags, commits, SHAs, etc. (if git log can handle it, so can this script)"
  echo "  Defaults to: last tag..HEAD (or first commit..HEAD if no tags)"
  exit 0
fi

if (( $# == 0 )); then
  mapfile -t tags < <(git for-each-ref \
    --sort=-creatordate \
    --format '%(refname:short)' \
    refs/tags)

  if (( ${#tags[@]} == 0 )); then
    from=$(git rev-list --max-parents=0 HEAD)
  else
    from=${tags[0]}
  fi
  to=HEAD

elif (( $# == 1 )); then
  from=$1
  to=HEAD

elif (( $# == 2 )); then
  from=$1
  to=$2

fi

git --no-pager log "${from}..${to}" --pretty=format:'- %h %s'
echo
