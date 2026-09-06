#!/usr/bin/env bash
# Upload a directory of Python distributions to PyPI, in the order that gets a working
# install soonest, and stop the moment PyPI refuses to create another project.
#
# PyPI caps how many new projects an account may create in a window, and a release that
# introduces a package per capability runs past it. Retrying inside the cap does not help,
# so this makes one pass: files for projects that already exist go first, since adding a
# file to a project is never capped; then the projects that do not exist yet, most
# important first, so `pip install pamoja` works as early as possible; and the first
# "too many new projects" answer ends the pass. Whatever is left is printed for the
# scheduled backfill, which runs this again until nothing is left.
#
#   pypi-upload.sh <dist directory> <version>
#
# Exits 0 when every distribution is on PyPI or the only ones missing were capped, and 1
# when an upload failed for any other reason.
set -euo pipefail

dist="${1:?dist directory}"
version="${2:?version}"

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
for file in "${ordered[@]}"; do
  project=$(project_of "$file")
  if [ "${#capped[@]}" -gt 0 ] && ! exists "$project"; then
    capped+=("$project")
    continue
  fi
  echo "uploading $(basename "$file")"
  if output=$(twine upload --skip-existing --non-interactive "$file" 2>&1); then
    continue
  fi
  echo "$output"
  if grep -qiE "Too many new projects|429 Too Many" <<<"$output"; then
    echo "PyPI has capped new projects for now; the rest wait for the backfill."
    capped+=("$project")
  else
    echo "::error::uploading $(basename "$file") failed for a reason other than the cap"
    exit 1
  fi
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
