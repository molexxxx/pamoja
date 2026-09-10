# Terms

pamoja is free software and this site is its documentation. There is no service to sign up
for, no account, and nothing to buy, so these terms are short. They say what you may do
with the software and with these pages, and what the project does and does not promise.

## The software

Every crate and every package is under the [MIT license](https://github.com/molexxxx/pamoja/blob/main/LICENSE-MIT).
You may use it, copy it, change it, build on it, and redistribute it, in a commercial
product or anywhere else, with one condition: keep the copyright notice and the permission
notice with it.

The MIT license also disclaims, in its own words and in capitals, every warranty and all
liability. On most projects that paragraph is boilerplate that nobody reads. On this one it
matters, so the next section says why in plain language.

## What this software is not

pamoja drives things that move and things people depend on. The crates frame CAN and J1939
for vehicles, speak MAVLink to autopilots, publish `cmd_vel` to robots, open valves, pulse
servos and steppers, and carry readings from cold chains and water points. Software that
does those things can hurt someone when it is wrong.

Nothing in this project is certified for safety-critical, life-support, medical,
aviation, automotive, or nuclear use, and none of it has been assessed against a
functional-safety standard. It carries no certification, no approval, and no assessment by
any body.

Where a capability implements a published standard, the tests are pinned to that
standard's own reference vectors, MAVLink is checked against live ArduPilot and PX4
simulators, and the ROS 2 bridge is checked against ROS 2 Jazzy. That is evidence the
implementation matches the specification. It is not a claim that the specification, the
implementation, or your use of either is safe for a given deployment.

If what you are building can injure a person, damage property, or fail dangerously, the
assessment, the redundancy, the independent safety layer, and the sign-off are yours to
do. Read the code before you trust it, and test it in your own conditions.

## The documentation and this site

The pages, the figures, the tables, and the examples are part of the same repository under
the same MIT license. Quote them, translate them, print them, and build on them, keeping
the notice with what you take.

The examples are the exception worth naming, because they are not illustrations. Each one
is spliced from a test that runs in continuous integration on every change, so an example
here is the code that ran. That makes it accurate. It does not make it fit for your
hardware, your link, or your load.

## The name and the mark

The MIT license covers the code, not the identity. The name pamoja, the logotype, and the
node mark are the project's, and the license does not grant permission to use them as your
own product name or brand.

You are welcome, without asking, to use the name to say what your work does: that it uses
pamoja, is built on pamoja, is compatible with pamoja, or is a fork of it. Please do not
use the name or the mark in a way that suggests the project made, endorses, or supports
something it did not.

## Components the project did not write

Two typefaces are served from this site under the SIL Open Font License 1.1, and the
packages carry third-party dependencies under their own licenses. The
[notices page](notices.md) names them and links their terms.

## Availability

The site and the demo are published from a repository to a static host, free of charge,
with no promise of uptime and no support commitment. They may change or go away.

The registries are the durable copy. A version published to crates.io, npm, PyPI, or NuGet
stays there under that registry's own policy, whatever happens to this site.

## Links away from here

A link to GitHub, a registry, a standards body, or a parts vendor is a pointer, not an
endorsement, and what happens on the other side is under someone else's terms.

## Changes

These terms change by a commit, like the rest of the site, and the revision in the footer
is the version you are reading. Your rights to any version of the software you already
have come from the MIT license that shipped with it, and nothing on this page takes them
away.
