# Standards and conformance

Anything defined by a published standard is implemented from the authoritative
specification itself, and its tests are anchored to that specification's own
reference vectors. Bit layouts, field orders, reserved bits, and algorithm constants
are where the subtle bugs hide, and a plausible guess is worse than none.

Every row below links two things: the document, and the test in this repository that
holds the code to it. `cargo xtask links` fetches each document on every run, so a
specification that moves fails the build rather than sitting here looking authoritative.
A handful of publishers refuse scripted clients; those rows say so, and a person opens
them instead.

What a row's second heading claims:

| Anchored to | What the test asserts |
| --- | --- |
| Published vector | The specification's own test vector or worked example |
| Specification rule | Its constants, bit layout or equation, asserted directly |
| Live implementation | A third-party implementation the test talks to |
| Round trip only | The code against itself, with no external vector to pin against |

<!-- table: standards -->
<p class="source">50 standards registered, 29 of them pinned to the document's own published vectors. Counted from <code>docs/standards.toml</code> when this page was rendered.</p>

## Cryptography

The primitives under device identity, the secured session, and signed updates.

<nav class="hw-index" aria-label="Cryptography index"><a href="#fips-197">FIPS 197-upd1</a><a href="#rfc-4493">RFC 4493</a><a href="#fips-180-4">FIPS 180-4</a><a href="#rfc-2104">RFC 2104</a><a href="#rfc-4231">RFC 4231</a><a href="#rfc-5869">RFC 5869</a><a href="#rfc-7748">RFC 7748</a><a href="#rfc-8439">RFC 8439</a><a href="#rfc-9124">RFC 9124</a><a href="#rfc-9019">RFC 9019</a></nav>
<div class="hw-cards">
<article class="hw-card" aria-labelledby="fips-197">
<header class="hw-head">

### FIPS 197-upd1 {#fips-197}

<p class="hw-by">NIST</p>
<p class="hw-summary">AES-128, the block cipher under the LoRaWAN MIC and session keys</p>
</header>
<p class="hw-summary">Appendix C.1. The cipher itself is the RustCrypto `aes` crate; the vector is asserted through this crate's wrapper.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://csrc.nist.gov/pubs/fips/197/final"><span class="hw-main"><b>The document</b><small>FIPS 197-upd1</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-lorawan/src/crypto.rs#L113"><span class="hw-main"><b>The test</b><small><code>crypto.rs</code> line 113</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rfc-4493">
<header class="hw-head">

### RFC 4493 {#rfc-4493}

<p class="hw-by">IETF</p>
<p class="hw-summary">AES-CMAC, the message integrity code on every LoRaWAN frame</p>
</header>
<p class="hw-summary">All four worked examples in section 4, including the 40 and 64 byte cases.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.rfc-editor.org/info/rfc4493"><span class="hw-main"><b>The document</b><small>RFC 4493</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-lorawan/src/crypto.rs#L136"><span class="hw-main"><b>The test</b><small><code>crypto.rs</code> line 136</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="fips-180-4">
<header class="hw-head">

### FIPS 180-4 {#fips-180-4}

<p class="hw-by">NIST</p>
<p class="hw-summary">SHA-256, under MAVLink signing, the audit chain and the dashboard pairing channel</p>
</header>
<p class="hw-summary">Both Appendix B worked examples, against the hand-written implementation the browser uses.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://csrc.nist.gov/pubs/fips/180-4/upd1/final"><span class="hw-main"><b>The document</b><small>FIPS 180-4</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-dashboard/web/app/lib/crypto/vectors.test.mjs#L28"><span class="hw-main"><b>The test</b><small><code>vectors.test.mjs</code> line 28</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rfc-2104">
<header class="hw-head">

### RFC 2104 {#rfc-2104}

<p class="hw-by">IETF</p>
<p class="hw-summary">HMAC, the construction the session key schedule is built on</p>
</header>
<p class="hw-summary">Pinned through RFC 4231, which publishes the SHA-256 vectors RFC 2104 does not.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.rfc-editor.org/info/rfc2104"><span class="hw-main"><b>The document</b><small>RFC 2104</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-session/src/kdf.rs#L65"><span class="hw-main"><b>The test</b><small><code>kdf.rs</code> line 65</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rfc-4231">
<header class="hw-head">

### RFC 4231 {#rfc-4231}

<p class="hw-by">IETF</p>
<p class="hw-summary">The published HMAC-SHA-256 test vectors</p>
</header>
<p class="hw-summary">Cases 1 and 2 in Rust, cases 1, 2 and 6 in the browser implementation.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.rfc-editor.org/info/rfc4231"><span class="hw-main"><b>The document</b><small>RFC 4231</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-session/src/kdf.rs#L65"><span class="hw-main"><b>The test</b><small><code>kdf.rs</code> line 65</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rfc-5869">
<header class="hw-head">

### RFC 5869 {#rfc-5869}

<p class="hw-by">IETF</p>
<p class="hw-summary">HKDF, which derives the session keys from the shared secret</p>
</header>
<p class="hw-summary">The basic vector in Appendix A.1.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.rfc-editor.org/info/rfc5869"><span class="hw-main"><b>The document</b><small>RFC 5869</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-session/src/kdf.rs#L89"><span class="hw-main"><b>The test</b><small><code>kdf.rs</code> line 89</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rfc-7748">
<header class="hw-head">

### RFC 7748 {#rfc-7748}

<p class="hw-by">IRTF</p>
<p class="hw-summary">X25519, the key agreement that opens a secured session</p>
</header>
<p class="hw-summary">The Alice and Bob example in section 6.1, both public keys and the shared secret.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.rfc-editor.org/info/rfc7748"><span class="hw-main"><b>The document</b><small>RFC 7748</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-session/src/kex.rs#L171"><span class="hw-main"><b>The test</b><small><code>kex.rs</code> line 171</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rfc-8439">
<header class="hw-head">

### RFC 8439 {#rfc-8439}

<p class="hw-by">IRTF</p>
<p class="hw-summary">ChaCha20-Poly1305, which carries every sealed frame</p>
</header>
<p class="hw-summary">The worked example in section 2.8.2, sealed and opened.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.rfc-editor.org/info/rfc8439"><span class="hw-main"><b>The document</b><small>RFC 8439</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-session/src/aead.rs#L92"><span class="hw-main"><b>The test</b><small><code>aead.rs</code> line 92</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rfc-9124">
<header class="hw-head">

### RFC 9124 {#rfc-9124}

<p class="hw-by">IETF</p>
<p class="hw-summary">The information model a firmware manifest must answer</p>
</header>
<p class="hw-summary">Each named threat is answered by a test named after it. The SUIT CBOR serialization is a deliberate deviation: draft-ietf-suit-manifest is still a draft.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.rfc-editor.org/info/rfc9124"><span class="hw-main"><b>The document</b><small>RFC 9124</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-update/tests/rules.rs#L356"><span class="hw-main"><b>The test</b><small><code>rules.rs</code> line 356</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rfc-9019">
<header class="hw-head">

### RFC 9019 {#rfc-9019}

<p class="hw-by">IETF</p>
<p class="hw-summary">The firmware update architecture the update layer is built to</p>
</header>
<div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.rfc-editor.org/info/rfc9019"><span class="hw-main"><b>The document</b><small>RFC 9019</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-update/src/lib.rs#L39"><span class="hw-main"><b>The test</b><small><code>lib.rs</code> line 39</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
</div>

## Identity and signing

What names a device and what proves a message came from it.

<nav class="hw-index" aria-label="Identity and signing index"><a href="#rfc-8032">RFC 8032</a></nav>
<div class="hw-cards">
<article class="hw-card" aria-labelledby="rfc-8032">
<header class="hw-head">

### RFC 8032 {#rfc-8032}

<p class="hw-by">IRTF</p>
<p class="hw-summary">Ed25519, which signs a device identity, a telemetry batch, an audit entry and a release</p>
</header>
<p class="hw-summary">TEST 2 in section 7.1, asserting the full 64-byte signature.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.rfc-editor.org/info/rfc8032"><span class="hw-main"><b>The document</b><small>RFC 8032</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-security/src/identity.rs#L222"><span class="hw-main"><b>The test</b><small><code>identity.rs</code> line 222</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
</div>

## Encodings

The wire formats readings and manifests are written in.

<nav class="hw-index" aria-label="Encodings index"><a href="#rfc-8949">RFC 8949</a><a href="#protobuf-varint">Protocol Buffers encoding</a><a href="#rfc-4648">RFC 4648</a></nav>
<div class="hw-cards">
<article class="hw-card" aria-labelledby="rfc-8949">
<header class="hw-head">

### RFC 8949 {#rfc-8949}

<p class="hw-by">IETF</p>
<p class="hw-summary">CBOR, the encoding a signed update manifest is written in</p>
</header>
<p class="hw-summary">The Appendix A vectors, plus the shortest-form rule in 4.2.1 and the refusal of indefinite-length items.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.rfc-editor.org/info/rfc8949"><span class="hw-main"><b>The document</b><small>RFC 8949</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-update/src/cbor.rs#L203"><span class="hw-main"><b>The test</b><small><code>cbor.rs</code> line 203</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="protobuf-varint">
<header class="hw-head">

### Protocol Buffers encoding {#protobuf-varint}

<p class="hw-by">Google</p>
<p class="hw-summary">LEB128 varints and zigzag mapping, under the delta codec</p>
</header>
<p class="hw-summary">The published signed-integer mapping, asserted value by value.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://protobuf.dev/programming-guides/encoding/"><span class="hw-main"><b>The document</b><small>Protocol Buffers encoding</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-codec/src/delta.rs#L268"><span class="hw-main"><b>The test</b><small><code>delta.rs</code> line 268</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rfc-4648">
<header class="hw-head">

### RFC 4648 {#rfc-4648}

<p class="hw-by">IETF</p>
<p class="hw-summary">base64, which carries a radio payload through the packet forwarder</p>
</header>
<p class="hw-summary">All seven vectors in section 10, encoded and decoded, plus the unpadded form.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.rfc-editor.org/info/rfc4648"><span class="hw-main"><b>The document</b><small>RFC 4648</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-gateway/src/base64.rs#L160"><span class="hw-main"><b>The test</b><small><code>base64.rs</code> line 160</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
</div>

## Field I/O

The buses and framings that reach a part on the end of a wire.

<nav class="hw-index" aria-label="Field I/O index"><a href="#rfc-1055">RFC 1055, STD 47</a><a href="#cobs">Cheshire and Baker 1999</a><a href="#crc-16-modbus">CRC-16/MODBUS</a><a href="#modbus-serial">Modbus over Serial Line V1.02</a><a href="#iso-11898-1">ISO 11898-1:2024</a><a href="#sae-j1939-21">SAE J1939-21</a><a href="#um10204">UM10204 Rev. 7.0</a><a href="#crc-8-maxim">CRC-8/MAXIM-DOW</a></nav>
<div class="hw-cards">
<article class="hw-card" aria-labelledby="rfc-1055">
<header class="hw-head">

### RFC 1055, STD 47 {#rfc-1055}

<p class="hw-by">IETF</p>
<p class="hw-summary">SLIP framing over a serial line</p>
</header>
<p class="hw-summary">The octal constants and the stuffing rule, asserted as the RFC prints them.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.rfc-editor.org/info/rfc1055"><span class="hw-main"><b>The document</b><small>RFC 1055, STD 47</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-serial/src/slip.rs#L308"><span class="hw-main"><b>The test</b><small><code>slip.rs</code> line 308</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="cobs">
<header class="hw-head">

### Cheshire and Baker 1999 {#cobs}

<p class="hw-by">IEEE/ACM Transactions on Networking 7(2)</p>
<p class="hw-summary">Consistent Overhead Byte Stuffing</p>
</header>
<p class="hw-summary">COBS has no published standard; the paper is the authority. All eleven of its example encodings are asserted.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://ieeexplore.ieee.org/document/769765"><span class="hw-main"><b>The document</b><small>Cheshire and Baker 1999</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-serial/src/cobs.rs#L415"><span class="hw-main"><b>The test</b><small><code>cobs.rs</code> line 415</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="crc-16-modbus">
<header class="hw-head">

### CRC-16/MODBUS {#crc-16-modbus}

<p class="hw-by">RevEng CRC catalogue</p>
<p class="hw-summary">The frame check on every Modbus RTU frame</p>
</header>
<p class="hw-summary">Check value 0x4B37 over the ASCII digits, plus a real six-byte request frame.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://reveng.sourceforge.io/crc-catalogue/16.htm#crc.cat.crc-16-modbus"><span class="hw-main"><b>The document</b><small>CRC-16/MODBUS</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-modbus/src/crc.rs#L48"><span class="hw-main"><b>The test</b><small><code>crc.rs</code> line 48</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="modbus-serial">
<header class="hw-head">

### Modbus over Serial Line V1.02 {#modbus-serial}

<p class="hw-by">Modbus Organization</p>
<p class="hw-summary">The RTU frame, its function codes and its exception responses</p>
</header>
<p class="hw-summary">The specification's own request and response examples.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.modbus.org/docs/Modbus_over_serial_line_V1_02.pdf"><span class="hw-main"><b>The document</b><small>Modbus over Serial Line V1.02</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-modbus/src/pdu.rs#L337"><span class="hw-main"><b>The test</b><small><code>pdu.rs</code> line 337</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="iso-11898-1">
<header class="hw-head">

### ISO 11898-1:2024 {#iso-11898-1}

<p class="hw-by">ISO</p>
<p class="hw-summary">The CAN data link layer, classical and CAN FD</p>
</header>
<p class="hw-summary">Edition 3 supersedes the 2015 edition and the 1991 Bosch CAN 2.0 document.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.iso.org/standard/86384.html"><span class="hw-main"><b>The document</b><small>ISO 11898-1:2024</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Round trip only</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-can/src/frame.rs#L1"><span class="hw-main"><b>The test</b><small><code>frame.rs</code> line 1</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="sae-j1939-21">
<header class="hw-head">

### SAE J1939-21 {#sae-j1939-21}

<p class="hw-by">SAE International</p>
<p class="hw-summary">The J1939 data link layer: PGNs, the data page bit, and the PDU1 and PDU2 boundary</p>
</header>
<p class="hw-summary">Real published PGNs, including EEC1 at 61444 and the request PGN at 59904.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.sae.org/standards/content/j1939/21_202409/"><span class="hw-main"><b>The document</b><small>SAE J1939-21</small><small class="hw-note">sae.org serves a JavaScript shell to a scripted client, so a person opens this one.</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-can/src/j1939.rs#L226"><span class="hw-main"><b>The test</b><small><code>j1939.rs</code> line 226</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="um10204">
<header class="hw-head">

### UM10204 Rev. 7.0 {#um10204}

<p class="hw-by">NXP</p>
<p class="hw-summary">The I2C bus: addressing, the ten-bit form, and the reserved ranges</p>
</header>
<p class="hw-summary">The manual's own ten-bit worked example, 0x2A5 framing as 0xF4 0xA5.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.nxp.com/docs/en/user-guide/UM10204.pdf"><span class="hw-main"><b>The document</b><small>UM10204 Rev. 7.0</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-gpio/src/i2c.rs#L317"><span class="hw-main"><b>The test</b><small><code>i2c.rs</code> line 317</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="crc-8-maxim">
<header class="hw-head">

### CRC-8/MAXIM-DOW {#crc-8-maxim}

<p class="hw-by">RevEng CRC catalogue</p>
<p class="hw-summary">The 1-Wire check byte, on every DS18B20 scratchpad and ROM code</p>
</header>
<div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://reveng.sourceforge.io/crc-catalogue/1-15.htm#crc.cat.crc-8-maxim-dow"><span class="hw-main"><b>The document</b><small>CRC-8/MAXIM-DOW</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/ds18b20.rs#L720"><span class="hw-main"><b>The test</b><small><code>ds18b20.rs</code> line 720</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
</div>

## Sensors and actuators

Parts decoded byte for byte, each from the manufacturer's own datasheet.

<nav class="hw-index" aria-label="Sensors and actuators index"><a href="#crc-8-sensirion">Sensirion CRC-8</a><a href="#sensor-datasheets">Twelve manufacturer datasheets</a></nav>
<div class="hw-cards">
<article class="hw-card" aria-labelledby="crc-8-sensirion">
<header class="hw-head">

### Sensirion CRC-8 {#crc-8-sensirion}

<p class="hw-by">Sensirion</p>
<p class="hw-summary">The check byte on every SHT3x and SCD4x word</p>
</header>
<p class="hw-summary">The datasheets' own worked examples, including every CRC in the SCD4x command tables.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://sensirion.com/products/catalog/SHT31-DIS-B"><span class="hw-main"><b>The document</b><small>Sensirion CRC-8</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/sht3x.rs#L739"><span class="hw-main"><b>The test</b><small><code>sht3x.rs</code> line 739</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="sensor-datasheets">
<header class="hw-head">

### Twelve manufacturer datasheets {#sensor-datasheets}

<p class="hw-by">Bosch, Analog Devices, Texas Instruments, Sensirion, NXP</p>
<p class="hw-summary">BME280, BMP280, DS18B20, HDC1080, INA219, INA226, ADS1115, OPT3001, SCD4x, SHT3x, TMP117 and PCA9685</p>
</header>
<p class="hw-summary">Each driver asserts its datasheet's own worked compensation example. Every document is linked from the hardware page, part by part.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/hardware.html"><span class="hw-main"><b>The document</b><small>Twelve manufacturer datasheets</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/bme280.rs#L981"><span class="hw-main"><b>The test</b><small><code>bme280.rs</code> line 981</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
</div>

## Radio, mesh and gateways

Reaching a network that is not there, and the rules a regulator sets for doing it.

<nav class="hw-index" aria-label="Radio, mesh and gateways index"><a href="#ts001-1-0-4">TS001-1.0.4</a><a href="#rp002-1-0-5">RP002-1.0.5</a><a href="#itu-r-p525-5">ITU-R P.525-5</a><a href="#itu-r-p526-16">ITU-R P.526-16</a><a href="#an1200-22">AN1200.22</a><a href="#sx126x">SX1261/SX1262 datasheet, Rev 2.2</a><a href="#sx127x">SX1276/77/78/79 datasheet, Rev 7</a><a href="#packet-forwarder">PROTOCOL.TXT</a><a href="#basics-station">LoRa Basics Station 2.0.6</a><a href="#crc-16-ibm-3740">CRC-16/IBM-3740</a><a href="#cfr-15-247">47 CFR 15.247</a><a href="#en-300-220-2">ETSI EN 300 220-2 V3.3.1</a><a href="#erc-70-03">CEPT ERC Recommendation 70-03</a><a href="#itu-t-k27">ITU-T K.27</a></nav>
<div class="hw-cards">
<article class="hw-card" aria-labelledby="ts001-1-0-4">
<header class="hw-head">

### TS001-1.0.4 {#ts001-1-0-4}

<p class="hw-by">LoRa Alliance</p>
<p class="hw-summary">The LoRaWAN L2 specification: MAC framing and the OTAA join</p>
</header>
<p class="hw-summary">The join accept field positions in tables 44 and 55.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://resources.lora-alliance.org/technical-specifications/ts001-1-0-4-lorawan-l2-1-0-4-specification"><span class="hw-main"><b>The document</b><small>TS001-1.0.4</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-gateway/src/network.rs#L870"><span class="hw-main"><b>The test</b><small><code>network.rs</code> line 870</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rp002-1-0-5">
<header class="hw-head">

### RP002-1.0.5 {#rp002-1-0-5}

<p class="hw-by">LoRa Alliance</p>
<p class="hw-summary">The regional parameters for nine channel plans</p>
</header>
<p class="hw-summary">Each test is named after the table it checks: data rates, maximum payloads, the RX1 offset matrix.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://resources.lora-alliance.org/technical-specifications/rp002-1-0-5-lorawan-regional-parameters"><span class="hw-main"><b>The document</b><small>RP002-1.0.5</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-lora/src/region/tests.rs#L15"><span class="hw-main"><b>The test</b><small><code>tests.rs</code> line 15</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="itu-r-p525-5">
<header class="hw-head">

### ITU-R P.525-5 {#itu-r-p525-5}

<p class="hw-by">ITU-R</p>
<p class="hw-summary">Free-space attenuation, under the link budget</p>
</header>
<p class="hw-summary">Equations 5 and 6, reproduced and checked.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.itu.int/rec/R-REC-P.525-5-202411-I/en"><span class="hw-main"><b>The document</b><small>ITU-R P.525-5</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-lora/src/budget.rs#L773"><span class="hw-main"><b>The test</b><small><code>budget.rs</code> line 773</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="itu-r-p526-16">
<header class="hw-head">

### ITU-R P.526-16 {#itu-r-p526-16}

<p class="hw-by">ITU-R</p>
<p class="hw-summary">Propagation by diffraction, under the Fresnel zone clearance</p>
</header>
<p class="hw-summary">Equations 2 and 3.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.itu.int/rec/R-REC-P.526-16-202511-I/en"><span class="hw-main"><b>The document</b><small>ITU-R P.526-16</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-lora/src/budget.rs#L835"><span class="hw-main"><b>The test</b><small><code>budget.rs</code> line 835</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="an1200-22">
<header class="hw-head">

### AN1200.22 {#an1200-22}

<p class="hw-by">Semtech</p>
<p class="hw-summary">LoRa modulation: the noise floor and the demodulator SNR per spreading factor</p>
</header>
<div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.semtech.com/products/wireless-rf/lora-connect/sx1276"><span class="hw-main"><b>The document</b><small>AN1200.22</small><small class="hw-note">Semtech serves its documents through share links that render page images, so a person opens this one.</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-lora/src/budget.rs#L673"><span class="hw-main"><b>The test</b><small><code>budget.rs</code> line 673</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="sx126x">
<header class="hw-head">

### SX1261/SX1262 datasheet, Rev 2.2 {#sx126x}

<p class="hw-by">Semtech</p>
<p class="hw-summary">The sub-GHz transceiver: its command set, its IRQ map and its calibration sequences</p>
</header>
<div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.semtech.com/products/wireless-rf/lora-connect/sx1262"><span class="hw-main"><b>The document</b><small>SX1261/SX1262 datasheet, Rev 2.2</small><small class="hw-note">Semtech serves its documents through share links that render page images, so a person opens this one.</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-radios/src/sx126x/command.rs#L768"><span class="hw-main"><b>The test</b><small><code>command.rs</code> line 768</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="sx127x">
<header class="hw-head">

### SX1276/77/78/79 datasheet, Rev 7 {#sx127x}

<p class="hw-by">Semtech</p>
<p class="hw-summary">The earlier sub-GHz transceiver, and the errata its driver works around</p>
</header>
<div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.semtech.com/products/wireless-rf/lora-connect/sx1276"><span class="hw-main"><b>The document</b><small>SX1276/77/78/79 datasheet, Rev 7</small><small class="hw-note">Semtech serves its documents through share links that render page images, so a person opens this one.</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-radios/src/sx127x/config.rs#L809"><span class="hw-main"><b>The test</b><small><code>config.rs</code> line 809</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="packet-forwarder">
<header class="hw-head">

### PROTOCOL.TXT {#packet-forwarder}

<p class="hw-by">Semtech, Lora-net/packet_forwarder</p>
<p class="hw-summary">The UDP packet forwarder protocol, spoken from both the gateway and the server side</p>
</header>
<p class="hw-summary">The protocol's own printed example datagrams, and every TX_ACK error value.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/Lora-net/packet_forwarder/blob/master/PROTOCOL.TXT"><span class="hw-main"><b>The document</b><small>PROTOCOL.TXT</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-gateway/src/udp/payload.rs#L1088"><span class="hw-main"><b>The test</b><small><code>payload.rs</code> line 1088</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="basics-station">
<header class="hw-head">

### LoRa Basics Station 2.0.6 {#basics-station}

<p class="hw-by">Semtech</p>
<p class="hw-summary">The LNS protocol a gateway speaks to a network server</p>
</header>
<p class="hw-summary">The three ID6 identifier examples the protocol glossary prints.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://doc.sm.tc/station/"><span class="hw-main"><b>The document</b><small>LoRa Basics Station 2.0.6</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-gateway/src/station.rs#L1413"><span class="hw-main"><b>The test</b><small><code>station.rs</code> line 1413</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="crc-16-ibm-3740">
<header class="hw-head">

### CRC-16/IBM-3740 {#crc-16-ibm-3740}

<p class="hw-by">RevEng CRC catalogue</p>
<p class="hw-summary">The integrity check a mesh frame carries</p>
</header>
<p class="hw-summary">Check value 0x29B1. Also catalogued as CRC-16/CCITT-FALSE; the bare name CRC-16/CCITT means a different algorithm, KERMIT, which checks 0x2189.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://reveng.sourceforge.io/crc-catalogue/16.htm#crc.cat.crc-16-ibm-3740"><span class="hw-main"><b>The document</b><small>CRC-16/IBM-3740</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-mesh/src/crc.rs#L100"><span class="hw-main"><b>The test</b><small><code>crc.rs</code> line 100</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="cfr-15-247">
<header class="hw-head">

### 47 CFR 15.247 {#cfr-15-247}

<p class="hw-by">United States, FCC</p>
<p class="hw-summary">Conducted power, and the antenna gain allowance above 6 dBi</p>
</header>
<div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.ecfr.gov/current/title-47/chapter-I/subchapter-A/part-15/subpart-C/subject-group-ECFR2ea8a5f01db7c0b/section-15.247"><span class="hw-main"><b>The document</b><small>47 CFR 15.247</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-lora/src/budget.rs#L942"><span class="hw-main"><b>The test</b><small><code>budget.rs</code> line 942</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="en-300-220-2">
<header class="hw-head">

### ETSI EN 300 220-2 V3.3.1 {#en-300-220-2}

<p class="hw-by">ETSI</p>
<p class="hw-summary">The European short-range device limits the radio pages quote</p>
</header>
<div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.etsi.org/deliver/etsi_en/300200_300299/30022002/"><span class="hw-main"><b>The document</b><small>ETSI EN 300 220-2 V3.3.1</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/docs/radio.md#L359"><span class="hw-main"><b>The test</b><small><code>radio.md</code> line 359</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="erc-70-03">
<header class="hw-head">

### CEPT ERC Recommendation 70-03 {#erc-70-03}

<p class="hw-by">CEPT/ECC</p>
<p class="hw-summary">The duty cycle and power limits per European sub-band</p>
</header>
<p class="hw-summary">Revised often; the link resolves to the current edition rather than a dated one.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://docdb.cept.org/document/845"><span class="hw-main"><b>The document</b><small>CEPT ERC Recommendation 70-03</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/docs/radio.md#L294"><span class="hw-main"><b>The test</b><small><code>radio.md</code> line 294</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="itu-t-k27">
<header class="hw-head">

### ITU-T K.27 {#itu-t-k27}

<p class="hw-by">ITU-T</p>
<p class="hw-summary">Bonding and earthing inside a building, behind the mast guidance</p>
</header>
<p class="hw-summary">With IEC 62305-2 for the risk assessment and IEC 62305-3 for the protection system itself.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.itu.int/rec/T-REC-K.27/en"><span class="hw-main"><b>The document</b><small>ITU-T K.27</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/docs/radio.md#L254"><span class="hw-main"><b>The test</b><small><code>radio.md</code> line 254</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
</div>

## Drones

The protocol a ground station and a flight controller speak.

<nav class="hw-index" aria-label="Drones index"><a href="#mavlink-framing">MAVLink v1 and v2</a><a href="#crc-16-mcrf4xx">CRC-16/MCRF4XX</a><a href="#mavlink-crc-extra">CRC_EXTRA</a><a href="#mavlink-signing">MAVLink 2 signing</a></nav>
<div class="hw-cards">
<article class="hw-card" aria-labelledby="mavlink-framing">
<header class="hw-head">

### MAVLink v1 and v2 {#mavlink-framing}

<p class="hw-by">MAVLink project</p>
<p class="hw-summary">The frame layout, its magic bytes and its extension fields</p>
</header>
<div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://mavlink.io/en/guide/serialization.html"><span class="hw-main"><b>The document</b><small>MAVLink v1 and v2</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-mavlink/src/frame.rs#L505"><span class="hw-main"><b>The test</b><small><code>frame.rs</code> line 505</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="crc-16-mcrf4xx">
<header class="hw-head">

### CRC-16/MCRF4XX {#crc-16-mcrf4xx}

<p class="hw-by">RevEng CRC catalogue</p>
<p class="hw-summary">The frame checksum MAVLink calls X.25 but is not</p>
</header>
<p class="hw-summary">Check value 0x6F91, with a second test asserting it is not the X-25 value 0x906E.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://reveng.sourceforge.io/crc-catalogue/16.htm#crc.cat.crc-16-mcrf4xx"><span class="hw-main"><b>The document</b><small>CRC-16/MCRF4XX</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-mavlink/src/crc.rs#L154"><span class="hw-main"><b>The test</b><small><code>crc.rs</code> line 154</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="mavlink-crc-extra">
<header class="hw-head">

### CRC_EXTRA {#mavlink-crc-extra}

<p class="hw-by">MAVLink project</p>
<p class="hw-summary">The per-message byte that makes a dialect mismatch fail loudly</p>
</header>
<div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://mavlink.io/en/guide/serialization.html#crc_extra"><span class="hw-main"><b>The document</b><small>CRC_EXTRA</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-mavlink/src/crc.rs#L171"><span class="hw-main"><b>The test</b><small><code>crc.rs</code> line 171</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="mavlink-signing">
<header class="hw-head">

### MAVLink 2 signing {#mavlink-signing}

<p class="hw-by">MAVLink project</p>
<p class="hw-summary">The thirteen-byte signature and its replay window</p>
</header>
<div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://mavlink.io/en/guide/message_signing.html"><span class="hw-main"><b>The document</b><small>MAVLink 2 signing</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-mavlink/src/signing.rs#L335"><span class="hw-main"><b>The test</b><small><code>signing.rs</code> line 335</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
</div>

## Robotics

Naming, typing and carrying messages the way ROS 2 and Zenoh do.

<nav class="hw-index" aria-label="Robotics index"><a href="#ros2-names">ROS 2 topic and service names</a><a href="#rihs01">REP-2011, RIHS01</a><a href="#dds-xtypes">DDS-XTypes 1.3, clause 7.4</a><a href="#rmw-zenoh">rmw_zenoh key expressions</a><a href="#zenoh-keyexpr">Zenoh key expressions</a></nav>
<div class="hw-cards">
<article class="hw-card" aria-labelledby="ros2-names">
<header class="hw-head">

### ROS 2 topic and service names {#ros2-names}

<p class="hw-by">Open Robotics</p>
<p class="hw-summary">What a valid name is, and how it maps to a DDS topic</p>
</header>
<div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://design.ros2.org/articles/topic_and_service_names.html"><span class="hw-main"><b>The document</b><small>ROS 2 topic and service names</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-ros2/src/name.rs#L184"><span class="hw-main"><b>The test</b><small><code>name.rs</code> line 184</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rihs01">
<header class="hw-head">

### REP-2011, RIHS01 {#rihs01}

<p class="hw-by">ROS 2 project</p>
<p class="hw-summary">The type hash that tells two nodes they mean the same message</p>
</header>
<p class="hw-summary">The published hash for std_msgs/msg/String.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://ros.org/reps/rep-2011.html"><span class="hw-main"><b>The document</b><small>REP-2011, RIHS01</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-ros2/src/typehash.rs#L133"><span class="hw-main"><b>The test</b><small><code>typehash.rs</code> line 133</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="dds-xtypes">
<header class="hw-head">

### DDS-XTypes 1.3, clause 7.4 {#dds-xtypes}

<p class="hw-by">OMG</p>
<p class="hw-summary">Extended CDR, the encoding a ROS 2 message travels in</p>
</header>
<p class="hw-summary">Plain CDR is the CORBA transfer syntax; DDS and ROS 2 use the extended form.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.omg.org/spec/DDS-XTypes/1.3/"><span class="hw-main"><b>The document</b><small>DDS-XTypes 1.3, clause 7.4</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Round trip only</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-ros2/src/msg.rs#L324"><span class="hw-main"><b>The test</b><small><code>msg.rs</code> line 324</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rmw-zenoh">
<header class="hw-head">

### rmw_zenoh key expressions {#rmw-zenoh}

<p class="hw-by">ROS 2 project</p>
<p class="hw-summary">The key a ROS 2 topic becomes when Zenoh carries it</p>
</header>
<div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md"><span class="hw-main"><b>The document</b><small>rmw_zenoh key expressions</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-ros2/src/key.rs#L68"><span class="hw-main"><b>The test</b><small><code>key.rs</code> line 68</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="zenoh-keyexpr">
<header class="hw-head">

### Zenoh key expressions {#zenoh-keyexpr}

<p class="hw-by">Zenoh project</p>
<p class="hw-summary">Validity, canonical form, and how a wildcard matches</p>
</header>
<p class="hw-summary">The specification's own canonical-form reordering examples.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://zenoh.io/docs/manual/abstractions/"><span class="hw-main"><b>The document</b><small>Zenoh key expressions</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Published vector</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-zenoh/src/keyexpr.rs#L230"><span class="hw-main"><b>The test</b><small><code>keyexpr.rs</code> line 230</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
</div>

## Messaging

The protocols a node speaks to a broker or a server.

<nav class="hw-index" aria-label="Messaging index"><a href="#mqtt-5">MQTT Version 5.0</a><a href="#rfc-7252">RFC 7252</a><a href="#rfc-7641">RFC 7641</a></nav>
<div class="hw-cards">
<article class="hw-card" aria-labelledby="mqtt-5">
<header class="hw-head">

### MQTT Version 5.0 {#mqtt-5}

<p class="hw-by">OASIS</p>
<p class="hw-summary">Topic names, the single and multi-level wildcards, and the rule that a wildcard never matches a $-topic</p>
</header>
<p class="hw-summary">Sections 4.7.1 and 4.7.2. The client itself speaks 3.1.1, whose topic rules are the same.</p><div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901241"><span class="hw-main"><b>The document</b><small>MQTT Version 5.0</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Specification rule</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-core/src/transport.rs#L289"><span class="hw-main"><b>The test</b><small><code>transport.rs</code> line 289</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rfc-7252">
<header class="hw-head">

### RFC 7252 {#rfc-7252}

<p class="hw-by">IETF</p>
<p class="hw-summary">CoAP: the message types, the code space and the Uri-Path option</p>
</header>
<div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.rfc-editor.org/info/rfc7252"><span class="hw-main"><b>The document</b><small>RFC 7252</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Round trip only</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-coap/tests/roundtrip.rs#L99"><span class="hw-main"><b>The test</b><small><code>roundtrip.rs</code> line 99</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rfc-7641">
<header class="hw-head">

### RFC 7641 {#rfc-7641}

<p class="hw-by">IETF</p>
<p class="hw-summary">Observing a CoAP resource, so a server pushes a change</p>
</header>
<div class="hw-foot">
<section class="hw-buy"><h4>Read it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.rfc-editor.org/info/rfc7641"><span class="hw-main"><b>The document</b><small>RFC 7641</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
<section class="hw-learn"><h4>Round trip only</h4><ul class="hw-rows"><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-coap/src/lib.rs#L349"><span class="hw-main"><b>The test</b><small><code>lib.rs</code> line 349</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
</div>
<!-- end -->

## Across languages

A second set of vectors, in
[`conformance/vectors.json`](https://github.com/molexxxx/pamoja/blob/main/conformance/vectors.json),
does the same job across languages rather than against a specification. It is generated
from the Rust implementation by
[`cargo run -p pamoja-examples --example conformance_vectors`](https://github.com/molexxxx/pamoja/blob/main/examples/conformance_vectors.rs)
and asserted by all four binding suites, so a facade that drifts fails instead of quietly
returning something else. CI regenerates the file and fails on any difference.

Each vector pins the exact bytes where a capability has a wire format, and the register
values, tables and timings where it does not. The suites are checked by perturbation: each
runner mutates a vector it has loaded and asserts that its own comparison rejects it, so a
suite that silently stopped comparing fails instead of passing.

## Against the real thing

Where a specification has a live implementation to talk to, CI talks to it.

The MAVLink layer commands [ArduPilot](https://ardupilot.org/) and [PX4](https://px4.io/)
in SITL: it reads a heartbeat, requests a message, walks the mission protocol's receiver
state machine by downloading the vehicle's plan, uploads one of its own, and sends an arm
command. Each is a real `COMMAND_ACK` from the running autopilot. Where the autopilot has
mission storage the uploaded plan is read back and its item count asserted; the headless
ArduPilot build advertises none and answers `NO_SPACE`, which the test records rather than
asserts around.

The ROS 2 bridge exchanges topics, services and actions with
[ROS 2 Jazzy](https://docs.ros.org/en/jazzy/), and a further case carries a ROS 2
publication over Zenoh under `rmw_zenoh` with that RMW selected and a router running. The
LoRa gateway registers a device with a [ChirpStack](https://www.chirpstack.io/) network
server and forwards a join and an uplink through it.

Every `no_std` crate is cross-compiled for a Cortex-M4F microcontroller, since a host
`no_std` build still links the host `std`.
