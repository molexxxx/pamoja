#!/usr/bin/env bash
# Put a bot's change up as a pull request and merge it once every check passes.
#
# The working tree's changes to tracked files are committed to <branch> as the GH_TOKEN
# account, pushed, and opened as a pull request against main. Older open pull requests
# from the same bot, those whose branch shares the prefix up to <branch>'s last slash,
# are closed as superseded. Once every check on this one passes it is squash-merged.
#
# A pull request whose checks fail stays open as the report, and the run still succeeds.
# A scheduled run reports on main's latest commit, so failing it would mark main red for
# a change main never took. The pull request gets one comment, updated on each run, that
# links the failed checks and says what happens next, so an open bot pull request always
# explains itself.
#
#   merge-when-green.sh [--keep-open] <branch> <title> <body file> <label>
#
# With --keep-open, an open pull request already on <branch> is left as it is and only
# watched and merged, so a fix someone pushed to it is never overwritten. Without it, the
# branch is force-pushed with this run's change.
#
# GH_TOKEN must belong to an account that can push and merge; a merge made with the
# workflow's own token would not start main's CI.
#
# Exits 0 when the pull request merged or was left open with a failing check, and 1 when
# it could not be opened or no check ever registered on it.
set -euo pipefail

keep_open=0
if [ "${1:-}" = "--keep-open" ]; then
    keep_open=1
    shift
fi
branch="${1:?branch}"
title="${2:?title}"
body_file="${3:?body file}"
label="${4:?label}"
prefix="${branch%/*}/"
marker="<!-- merge-when-green -->"

report_failure() {
    local body existing
    body="$(mktemp)"
    {
        echo "$marker"
        echo "This pull request was not merged because a check failed:"
        echo
        gh pr checks "$url" --json name,bucket,link \
            --jq '.[] | select(.bucket == "fail" or .bucket == "cancel") | "- [\(.name)](\(.link))"'
        echo
        if [ "$keep_open" -eq 1 ]; then
            echo "A fix pushed to \`$branch\` is merged by the next run once every check passes."
        else
            echo "The next run replaces this pull request with a fresh one."
        fi
    } > "$body"
    existing="$(gh api --paginate "repos/$GITHUB_REPOSITORY/issues/${url##*/}/comments" \
        --jq ".[] | select(.body | startswith(\"$marker\")) | .id" | tail -n 1)"
    if [ -n "$existing" ]; then
        gh api -X PATCH "repos/$GITHUB_REPOSITORY/issues/comments/$existing" -F "body=@$body" > /dev/null
    else
        gh pr comment "$url" --body-file "$body" > /dev/null
    fi
}

url="$(gh pr list --head "$branch" --state open --json url --jq '.[0].url // empty')"

if [ "$keep_open" -eq 1 ] && [ -n "$url" ]; then
    echo "$url is already open; watching it instead of pushing over it"
else
    login="$(gh api user --jq .login)"
    git config user.name "$login"
    git config user.email "$(gh api user --jq .id)+${login}@users.noreply.github.com"
    git switch -c "$branch"
    git commit -am "$title"
    git push --force "https://x-access-token:${GH_TOKEN}@github.com/${GITHUB_REPOSITORY}.git" "$branch"
    if [ -z "$url" ]; then
        url="$(gh pr create --base main --head "$branch" --title "$title" --body-file "$body_file" --label "$label")"
    fi
fi

gh pr list --state open --search "head:$prefix" --json number,headRefName \
    --jq ".[] | select(.headRefName != \"$branch\") | .number" |
    while read -r old; do
        gh pr close "$old" --delete-branch --comment "Superseded by $url."
    done

# gh pr checks fails with "no checks reported" until it sees a check of its own, even
# when gh pr view already lists one, so the wait polls the same command.
registered=0
for _ in $(seq 1 30); do
    sleep 20
    registered="$(gh pr checks "$url" --json name --jq length 2>/dev/null || true)"
    [ "${registered:-0}" -gt 0 ] && break
done
if [ "${registered:-0}" -eq 0 ]; then
    echo "::error::no check registered on $url within ten minutes"
    exit 1
fi

if gh pr checks "$url" --watch --fail-fast --interval 60; then
    # The body GitHub generates adds a co-author line for any branch commit whose address
    # differs from the account's commit email, even for the same account.
    gh pr merge "$url" --squash --delete-branch --body ""
else
    report_failure
    echo "::warning::$url has a failing check, so it stays open for a person to read"
fi
