---
name: release
description: Cut a release of Bear -- verify metadata, run the full check set, fast-forward to master, tag, and publish a GitHub release plus discussion announcement. Use when the user asks to release, cut a version, tag a release, or ship a new Bear version.
---

# Release Bear

This skill captures the release process. The goal is that running through it
mechanically produces a tagged, published release with consistent metadata.

Replace `X.Y.Z` with the version being released (e.g. `4.1.2`). The release
branch is `X.Y.Z-rc`; the destination branch is `master`.

Pushing the tag and publishing the GitHub release are outward-facing and hard
to undo. Drive the steps below and confirm with the user before any
outward-facing action (the fast-forward to master, pushing the tag, creating
the release, posting the announcement).

## 1. Preconditions

- All work for the release is on `X.Y.Z-rc`, branched from `master`.
- `master` has no commits the rc branch is missing:
  ```sh
  git fetch origin
  git log --oneline HEAD..origin/master   # must be empty
  ```
  If non-empty, rebase the rc branch onto `master` first.
- Latest CI run on `X.Y.Z-rc` is green:
  ```sh
  gh run list --branch X.Y.Z-rc --limit 1
  ```

## 2. Pre-flight checklist

Run from the repo root on the rc branch:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --verbose          # integration tests need a debug build
cargo test
```

All four must pass before tagging.

Then verify the release metadata:

| Item | Where | Check |
|---|---|---|
| Workspace version | `Cargo.toml` (`[workspace.package]`) | matches `X.Y.Z` |
| Lockfile in sync | `Cargo.lock` | `bear`, `compilers-codegen`, etc. show `X.Y.Z`; clean `cargo build` does not modify it |
| Man page date | `man/bear.1.md` line 3 | set to today (e.g. `April 25, 2026`) |
| Man page generated | `man/bear.1` | regenerated from `.md` (see below) |

If the man page date or content changed, regenerate `bear.1`:

```sh
cd man && pandoc -s -t man bear.1.md -o bear.1
```

Commit any pre-flight fixups to `X.Y.Z-rc` and let CI run again.

## 3. Merge to master

Use a fast-forward merge to keep `master`'s first-parent history linear, which
is the convention the 4.x series follows.

```sh
git checkout master
git pull --ff-only origin master
git merge --ff-only X.Y.Z-rc
```

Do not create a merge commit. If `--ff-only` fails, the rc branch is behind
master - rebase it and start over.

## 4. Tag

Tags are unprefixed (`4.1.2`, not `v4.1.2`), annotated, and SSH-signed. The
message body uses the `v`-prefixed form.

Verify your signing config once:

```sh
git config --get gpg.format        # ssh
git config --get user.signingkey   # path or key starting with "key::"
git config --get tag.gpgsign       # true (recommended)
```

Then tag the merge tip:

```sh
git tag -s X.Y.Z -m "vX.Y.Z"
git tag --verify X.Y.Z             # confirm signature is good
```

## 5. Push

```sh
git push origin master
git push origin X.Y.Z
```

## 6. Draft release notes

Use prior releases as the template (`gh release view 4.1.1`). Sections, in
order, when applicable:

- `### Features`
- `### Bug Fixes`
- `### Performance`
- `### Internal Refactoring`
- `### Documentation`
- `### Closed Issues` - bullet list of `#NNN - one-line description`
- `### Thanks` - issue reporters and external PR authors, by `@handle`;
  plus the downstream packagers currently shipping Bear, by name or
  `@handle` where known (see the repology command below). Packagers are
  chronically invisible volunteers - name them every release.
- `### New Contributors` - first-time contributors only

Commit subjects are not enough on their own. A `refactor:` or `test:`
commit can still change something a user sees, typically the wording of a
diagnostic or the shape of a log line, and its subject will not say so.
Read the bodies, and read any rationale entry added since the previous
tag: those record accepted user-visible costs in their Consequences
section. Anything a user could have scripted against belongs in the notes
even when no feature changed.

Trailer:

```
**Full Changelog**: https://github.com/rizsotto/Bear/compare/PREV...X.Y.Z
```

Useful inputs while drafting:

```sh
# commit subjects since the previous tag
git log --pretty="%s" PREV..X.Y.Z

# rationale entries added since the previous tag; their Consequences
# sections name the user-visible deltas the subjects above will not
git log --diff-filter=A --name-only --pretty=format: PREV..X.Y.Z -- docs/rationale/

# external contributors since the previous tag
git log --pretty="%an <%ae>" PREV..X.Y.Z | sort -u

# issues closed since the previous tag
gh issue list --state closed --search "closed:>=YYYY-MM-DD" --limit 50

# PRs merged since the previous tag
gh pr list --state merged --search "merged:>=YYYY-MM-DD" --limit 50

# downstream packagers shipping the latest Bear (repology needs a User-Agent;
# "newest" entries are the repos that kept up with the previous release)
curl -s -A 'Bear-release-notes' 'https://repology.org/api/v1/project/bear-clang' |
    python3 -c "import json,sys; [print(p['repo'], p['version'], ', '.join(p.get('maintainers',[]))) for p in json.load(sys.stdin) if p.get('status')=='newest']"
```

Save the notes to a temporary file (e.g. `/tmp/release-notes.md`) - they are
reused for the GitHub release and the discussion announcement.

## 7. Publish the GitHub release

```sh
gh release create X.Y.Z \
    --title X.Y.Z \
    --notes-file /tmp/release-notes.md \
    --verify-tag
```

Use `--draft` first if you want to review the rendered output before it goes
live, then `gh release edit X.Y.Z --draft=false`.

## 8. Announce on the discussions thread

The pinned thread for release announcements is
<https://github.com/rizsotto/Bear/discussions/399>.

Keep the announcement short:

- One-line summary linking to the release page.
- Three to five highlight bullets.
- Thanks to issue reporters and external contributors by `@handle`.
- Invite users to open new issues for problems; the discussion thread is for
  general feedback.

Post with:

```sh
gh api graphql -f query='
  mutation($id: ID!, $body: String!) {
    addDiscussionComment(input: {discussionId: $id, body: $body}) {
      comment { url }
    }
  }' -f id=DISCUSSION_NODE_ID -f body="$(cat /tmp/announcement.md)"
```

(Get `DISCUSSION_NODE_ID` once via
`gh api graphql -f query='{ repository(owner:"rizsotto",name:"Bear") { discussion(number:399) { id } } }'`.)

Alternatively, paste through the web UI.

## 9. Post-release

- Pin the announcement comment in discussion #399 if you typically do.
- Verify the release page renders correctly and the tag signature is shown.
- Notify downstream packagers if the release contains packaging-relevant
  changes (install layout, prerequisites, breaking flags). Past channels:
  Homebrew, Arch, Debian/Fedora maintainers.
- Open a new branch `<next>-rc` when work for the next version begins, and
  bump `Cargo.toml` (`[workspace.package].version`) on that branch.

## Conventions reference

| Item | Convention |
|---|---|
| Release branch name | `X.Y.Z-rc` |
| Tag name | `X.Y.Z` (no `v` prefix) |
| Tag type | annotated, SSH-signed |
| Tag message | `vX.Y.Z` |
| GitHub release title | `X.Y.Z` |
| Merge style | fast-forward only |
| Version source of truth | `Cargo.toml` `[workspace.package].version` |
