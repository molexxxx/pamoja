# Privacy

This site does not collect anything about you. There are no cookies, no analytics, no tag
managers, no embedded players, no social widgets, and no fonts fetched from anyone else's
server. There is no account to create and no form to fill in.

What follows is all of it, written from what the code and the hosting actually do rather
than from a template. This page lives in the repository like every other page, so its
history is the record of every change ever made to it.

## What your browser stores

Two values, both in `localStorage`, both written only after you choose something:

| Key | Written when | What it holds | Leaves your device |
| --- | --- | --- | --- |
| `pamoja:scheme` | You use the light and dark control in the header | `light` or `dark` | No |
| `pamoja:lang` | You pin a language in a listing or in the ordering table | `rust`, `typescript`, `python`, or `c` | No |

Both are read only by the page you are on, so it can open the way you left it. Clearing
site data in your browser removes them, and every page works without them.

Nothing else is stored. No cookies are set, session storage is unused, and no database or
device fingerprint is written.

## What leaves your browser

Requests for this site's own files, and nothing else. The pages, the stylesheets, the two
typefaces, the scripts, the figures, and the search index all come from this domain. Open
your browser's network panel on any page here and the list of hosts contacted has exactly
one entry.

Search is the case worth spelling out, because most documentation sites send queries to
someone. Here the whole index is one file this site serves, your browser downloads it the
first time you focus the search box, and every keystroke after that is matched on your own
device. What you type is never transmitted.

## Who serves the site

`pamoja.molex.cloud` is a name for `molexxxx.github.io`, so every page is served by GitHub
Pages. GitHub is therefore the only party that sees a request, and it records what any web
server records: the address the request came from, the file asked for, the time, and the
browser's user agent. GitHub holds and uses those logs under its own terms, described in
the [GitHub Privacy Statement](https://docs.github.com/site-policy/privacy-policies/github-privacy-statement)
and the [GitHub Pages notice on visitor data](https://docs.github.com/pages/getting-started-with-github-pages/about-github-pages).
The project never receives those logs and has no way to ask for them.

There is no content delivery network, reverse proxy, or third-party edge in front of the
site. The domain resolves straight to GitHub's Pages addresses.

## The dashboard demo

The [dashboard demo](https://pamoja.molex.cloud/dashboard/) is the same local-first
dashboard the `pamoja-dashboard` crate serves, built to run with no backend. It asks this
same domain for a live device, gets nothing, and falls back to sample readings that ship
inside it. Every number you see there is generated data in your own browser. It reaches no
device of yours, no device of ours, and no server.

The dashboard remembers your language and layout choices the same way the site does, in
your own browser, and sends them nowhere.

## Links away from here

Pages link out to GitHub, crates.io, npm, PyPI, NuGet, docs.rs, and to the standards
bodies and vendors whose parts the hardware reference cites. Following a link takes you to
someone else's site, under someone else's policy. Nothing is prefetched, and no link is
wrapped in a redirector or a click tracker.

## What you choose to send the project

Nothing on this site sends anything. If you want to reach the project, you do it
deliberately, through one of these:

- **A GitHub issue or pull request.** Public, and hosted by GitHub, so what you write and
  the account you write it from are visible to anyone and covered by GitHub's policies.
- **A private security advisory.** Confidential between you and the maintainer until a fix
  ships. The [security policy](https://github.com/molexxxx/pamoja/blob/main/SECURITY.md)
  says what happens next.
- **Email to the maintainer**, for conduct reports or anything else that should not be
  public. Held only as long as the matter is open.

The project keeps what you send in the place you sent it and uses it only to answer you.
It is never added to a mailing list, sold, or passed to anyone else.

## Children

Nothing here is directed at children, and nothing is collected from anyone of any age.

## Changes

This page changes by a commit, like the rest of the site, and the revision the footer
carries is the version you are reading. If the site ever starts collecting something, this
page will say so before it does.
