# Notices and contact

Who holds the copyright, what the project uses that it did not write, and how to reach a
person.

## Copyright and license

Copyright 2026 Anthony Wiedman. Released under the
[MIT license](https://github.com/molexxxx/pamoja/blob/main/LICENSE-MIT), which travels
with every crate, every package, and this site. The [terms](terms.md) say what that means
in practice.

## The typefaces

Both faces are served from this site rather than from a font host, and their licenses are
served beside them.

| Typeface | Used for | License |
| --- | --- | --- |
| [Archivo](https://github.com/Omnibus-Type/Archivo) | Every heading, label, and run of reading text | [SIL Open Font License 1.1](/fonts/LICENSE-Archivo.txt) |
| [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono) | Anything measured, installed, or typed | [SIL Open Font License 1.1](/fonts/LICENSE-JetBrainsMono.txt) |

## Dependencies

The engine is written to need very little. A build that names one capability compiles that
crate, `pamoja-core`, and nothing else from outside the workspace, which the table on the
[install page](../install.md) measures per feature set.

What the wider builds do pull in is audited on every change. Continuous integration runs
`cargo-deny` over the graph a user actually installs, refusing a dependency whose license
is not on the allowed list and failing on any security advisory against it. The policy is
[`deny.toml`](https://github.com/molexxxx/pamoja/blob/main/deny.toml) in the repository,
and the licenses of the crates in any particular build are listed by `cargo deny list`
against your own lockfile.

The bindings add a runtime each: napi-rs for TypeScript, PyO3 for Python, and cbindgen
with P/Invoke for C#. Each carries its own license, declared in its own package.

## Ported code

The SX1302 and SX1303 concentrator support in `pamoja-radios` follows Semtech's
`sx1302_hal`: the register map, the shape of the SPI transfers, and the order the two
on-chip microcontrollers are loaded in. That work is BSD 3-Clause, whose second condition
asks that the notice be reproduced in the documentation of anything that redistributes it,
so it is reproduced here in full. The same file carries the notices for two libraries the
reference implementation uses, and both are included for the same reason.

```text
Copyright (c) 2019, SEMTECH S.A.
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:
	* Redistributions of source code must retain the above copyright
	  notice, this list of conditions and the following disclaimer.
	* Redistributions in binary form must reproduce the above copyright
	  notice, this list of conditions and the following disclaimer in the
	  documentation and/or other materials provided with the distribution.
	* Neither the name of the Semtech corporation nor the
	  names of its contributors may be used to endorse or promote products
	  derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL SEMTECH S.A. BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

The parson library, used by the reference packet forwarder:

```text
Parson ( http://kgabis.github.com/parson/ )
Copyright (c) 2012 Krzysztof Gabis

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
```

The tinyMT32 library, used by the reference hardware abstraction layer:

```text
Copyright (c) 2011 Mutsuo Saito, Makoto Matsumoto, Hiroshima
University and The University of Tokyo. All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:

    * Redistributions of source code must retain the above copyright
      notice, this list of conditions and the following disclaimer.
    * Redistributions in binary form must reproduce the above
      copyright notice, this list of conditions and the following
      disclaimer in the documentation and/or other materials provided
      with the distribution.
    * Neither the name of the Hiroshima University nor the names of
      its contributors may be used to endorse or promote products
      derived from this software without specific prior written
      permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

The firmware images the two on-chip microcontrollers run are Semtech's own, and are not
carried in this repository. A caller supplies them, which is why the loading sequence takes
the bytes rather than holding them.

## Standards and third-party names

The capabilities implement published specifications, and the hardware reference cites the
datasheets and standards the drivers were written from. Names such as Modbus, CAN, J1939,
LoRa, LoRaWAN, MAVLink, ROS 2, Zenoh, and the names of the parts and boards documented
here belong to their respective owners. They appear to say what a capability speaks to.
None of them is affiliated with this project, and none of them endorses it.

## Security

Report a suspected vulnerability privately, never in a public issue or pull request. The
[security policy](https://github.com/molexxxx/pamoja/blob/main/SECURITY.md) says what to
send and what happens next; the fastest route is
[a private advisory](https://github.com/molexxxx/pamoja/security/advisories/new) on the
repository.

## Conduct

The project follows a [code of conduct](https://github.com/molexxxx/pamoja/blob/main/CODE_OF_CONDUCT.md)
that applies to the repository, the issues, and anywhere the project is represented.

## Contact

There is one maintainer, and these are the ways to reach them.

| For | Where | Public |
| --- | --- | --- |
| A bug, a question, or a capability request | [Open an issue](https://github.com/molexxxx/pamoja/issues/new/choose) | Yes |
| A change you want to contribute | [Open a pull request](https://github.com/molexxxx/pamoja/pulls), after reading [CONTRIBUTING](https://github.com/molexxxx/pamoja/blob/main/CONTRIBUTING.md) | Yes |
| A suspected vulnerability | [A private advisory](https://github.com/molexxxx/pamoja/security/advisories/new) | No |
| A conduct report, or anything else private | <molex@sent.com> | No |

An issue is the right default. It is public, so the next person with the same question
finds the answer, and it is where the work is tracked.

## This page

Like every page here, this one is a file in the repository and changes by a commit. The
revision in the footer is the version you are reading.
