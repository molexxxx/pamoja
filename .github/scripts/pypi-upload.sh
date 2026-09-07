#!/usr/bin/env bash
# Upload a directory of Python distributions to PyPI, in the order that gets a working
# install soonest, waiting out the new-project cap when PyPI says it will lift soon.
#
# PyPI caps how many new projects an account may create in a window, and a release that
# introduces a package per capability runs past it. A refusal carries the policy that
# tripped and the seconds until it resets, so this makes one pass: files for projects
# that already exist go first, since adding a file to a project is never capped; then
# the projects that do not exist yet, most important first, so `pip install pamoja`
# works as early as possible. A "too many new projects" answer whose reset fits the
# wait budget is slept through and retried; one that does not ends the pass, and
# whatever is left is printed for the scheduled backfill, which runs this again. A file
# PyPI refuses for any other reason is reported with PyPI's full answer and skipped, so
# one bad file does not hold back the projects behind it; the pass still fails at the end.
#
#   pypi-upload.sh <dist directory> <version>
#
# PYPI_WAIT_BUDGET is the seconds this pass may spend waiting for the cap, 3600 default.
#
# Exits 0 when every distribution is on PyPI or the only ones missing were capped, and 1
# when an upload failed for any other reason.
set -euo pipefail

dist="${1:?dist directory}"
version="${2:?version}"
budget="${PYPI_WAIT_BUDGET:-3600}"
upload="$(dirname "$0")/pypi-upload.py"

# The project a distribution file belongs to, as PyPI names it.
project_of() {
  local file
  file=$(basename "$1")
  file="${file%%-${version}*}"
  echo "${file//_/-}"
}

exists() {
  curl -fs -o /dev/null "https://pypi.org/pypi/$1/json"
}

# Creation order for projects that do not exist yet: the compiled engine, which every
# other package depends on; the bundle and the engine surface; the six domains; then the
# capabilities alphabetically.
rank() {
  case "$1" in
    pamoja-native) echo 0 ;;
    pamoja) echo 1 ;;
    pamoja-core) echo 2 ;;
    pamoja-field-io|pamoja-sensing|pamoja-radio|pamoja-trust|pamoja-transports|pamoja-profiles) echo 3 ;;
    *) echo 4 ;;
  esac
}

existing=()
fresh=()
for file in "$dist"/*; do
  project=$(project_of "$file")
  if exists "$project"; then
    existing+=("$file")
  else
    fresh+=("$(rank "$project") $project $file")
  fi
done

ordered=("${existing[@]}")
if [ "${#fresh[@]}" -gt 0 ]; then
  while IFS= read -r line; do
    ordered+=("${line##* }")
  done < <(printf '%s\n' "${fresh[@]}" | sort -k1,1n -k2,2)
fi

capped=()
refused=()
for file in "${ordered[@]}"; do
  project=$(project_of "$file")
  if [ "${#capped[@]}" -gt 0 ] && ! exists "$project"; then
    capped+=("$project")
    continue
  fi
  while :; do
    echo "uploading $(basename "$file")"
    if output=$(python "$upload" "$file" 2>&1); then
      grep -E "^rate limit " <<<"$output" || true
      break
    fi
    echo "$output"
    if ! grep -qiE "Too many new projects|429 Too Many" <<<"$output"; then
      echo "::error::uploading $(basename "$file") failed for a reason other than the cap"
      refused+=("$(basename "$file")")
      break
    fi
    resets=$(sed -n 's/^pypi-reset-seconds: \([0-9]\{1,\}\)$/\1/p' <<<"$output" | tail -1)
    resets="${resets:-0}"
    # A margin over PyPI's own figure, since the window has to have moved past the
    # oldest creation for the slot to be there when the retry lands.
    wait=$((resets + 30))
    if [ "$resets" -gt 0 ] && [ "$wait" -le "$budget" ]; then
      echo "PyPI's new-project cap resets in ${resets}s; waiting for it."
      sleep "$wait"
      budget=$((budget - wait))
      continue
    fi
    if [ "$resets" -gt 0 ]; then
      echo "PyPI has capped new projects; it resets in ${resets}s, longer than this pass waits."
    else
      echo "PyPI has capped new projects and reported no reset; the rest wait for the backfill."
    fi
    capped+=("$project")
    break
  done
done

missing=()
for file in "$dist"/*; do
  project=$(project_of "$file")
  if ! curl -fs -o /dev/null "https://pypi.org/pypi/$project/$version/json"; then
    missing+=("$project")
  fi
done
mapfile -t missing < <(printf '%s\n' "${missing[@]}" | sort -u | sed '/^$/d')

total=$(ls "$dist" | sed "s/-${version}.*//" | tr '_' '-' | sort -u | wc -l)
echo "on PyPI at $version: $((total - ${#missing[@]})) of $total projects"
if [ "${#missing[@]}" -gt 0 ]; then
  printf '  missing: %s\n' "${missing[@]}"
  echo "::warning::${#missing[@]} project(s) are not on PyPI yet because PyPI caps new projects; the pypi-backfill workflow uploads them as the cap allows"
fi
if [ "${#refused[@]}" -gt 0 ]; then
  printf '  refused: %s\n' "${refused[@]}"
  exit 1
fi
