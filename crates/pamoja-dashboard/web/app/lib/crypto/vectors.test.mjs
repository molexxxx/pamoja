// vectors.test.mjs - the published vectors the browser-side crypto is pinned to.
//
// These primitives authenticate every control command the dashboard sends, and they are
// hand-written because window.crypto.subtle is unavailable over the plain-http hotspot.
// A round-trip against themselves would pass just as happily if they were wrong, so each
// one is checked against the vectors its own standard publishes, and the pairing
// exchange is checked against the same file the Rust node asserts, which is what proves
// the two halves derive the same key.
//
// Run with: node --test crates/pamoja-dashboard/web/app/lib/crypto

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

import { concat, toHex, utf8 } from './bytes.js';
import { hkdfSha256 } from './hkdf.js';
import { hmacSha256 } from './hmac.js';
import { sha256 } from './sha256.js';

const fromHex = (hex) =>
  new Uint8Array((hex.match(/../g) ?? []).map((pair) => parseInt(pair, 16)));

const repeat = (byte, count) => new Uint8Array(count).fill(byte);

// FIPS 180-4, the two worked examples in appendix B plus the empty message.
test('sha256 matches the examples in FIPS 180-4', () => {
  assert.equal(
    toHex(sha256(utf8('abc'))),
    'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad',
  );
  assert.equal(
    toHex(sha256(utf8('abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq'))),
    '248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1',
  );
  assert.equal(
    toHex(sha256(new Uint8Array(0))),
    'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
  );
});

// The lengths either side of the point where the 64-byte block no longer has room for
// the padding and the 8-byte length, which is where a hand-written SHA-256 goes wrong.
// Each digest is a repeated 'a' of that length.
test('sha256 pads correctly around the block boundary', () => {
  const digests = [
    [55, '9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318'],
    [56, 'b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a'],
    [63, '7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34'],
    [64, 'ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb'],
    [65, '635361c48bb9eab14198e76ea8ab7f1a41685d6ad62aa9146d301d4f17eb0ae0'],
    [119, '31eba51c313a5c08226adf18d4a359cfdfd8d2e816b13f4af952f7ea6584dcfb'],
    [120, '2f3d335432c70b580af0e8e1b3674a7c020d683aa5f73aaaedfdc55af904c21c'],
  ];
  for (const [length, digest] of digests) {
    assert.equal(toHex(sha256(repeat(0x61, length))), digest, `${length} bytes`);
  }
});

// The message is fed in as one array, so a caller that assembles it from pieces has to
// reach the same digest.
test('sha256 does not depend on how the message was assembled', () => {
  const whole = utf8('pamoja/dashboard/cmd v1');
  const pieces = concat(whole.subarray(0, 7), whole.subarray(7, 17), whole.subarray(17));
  assert.equal(toHex(sha256(pieces)), toHex(sha256(whole)));
});

// RFC 4231, section 4. Cases 1 and 2 are ordinary keys; case 6 is longer than the
// 64-byte block, which is the path that hashes the key first.
test('hmacSha256 matches the vectors in RFC 4231', () => {
  assert.equal(
    toHex(hmacSha256(repeat(0x0b, 20), utf8('Hi There'))),
    'b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7',
  );
  assert.equal(
    toHex(hmacSha256(utf8('Jefe'), utf8('what do ya want for nothing?'))),
    '5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843',
  );
  assert.equal(
    toHex(
      hmacSha256(repeat(0xaa, 131), utf8('Test Using Larger Than Block-Size Key - Hash Key First')),
    ),
    '60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54',
  );
});

// RFC 5869, appendix A. Case 1 is the basic one; case 3 has an empty salt and info,
// which is the branch that substitutes a block of zeros for the salt.
test('hkdfSha256 matches the vectors in RFC 5869', () => {
  assert.equal(
    toHex(
      hkdfSha256(
        fromHex('000102030405060708090a0b0c'),
        repeat(0x0b, 22),
        fromHex('f0f1f2f3f4f5f6f7f8f9'),
        42,
      ),
    ),
    '3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865',
  );
  assert.equal(
    toHex(hkdfSha256(new Uint8Array(0), repeat(0x0b, 22), new Uint8Array(0), 42)),
    '8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8',
  );
});

// The exchange itself, against the file src/auth.rs asserts. If either half changes how
// it derives the key, one of the two tests fails and the control channel does not
// silently stop interoperating.
test('the pairing exchange agrees with the node', () => {
  const vector = JSON.parse(
    readFileSync(fileURLToPath(new URL('../../../../tests/pairing-vector.json', import.meta.url)), 'utf8'),
  );
  const key = hkdfSha256(utf8(vector.nonce), utf8(vector.secret), utf8(vector.info), 32);
  assert.equal(toHex(key), vector.key);
  assert.equal(
    toHex(hmacSha256(key, utf8('confirm\n' + vector.sessionId))),
    vector.confirmMac,
  );
  assert.equal(
    toHex(hmacSha256(key, utf8(vector.command.counter + '\n' + vector.command.payload))),
    vector.command.mac,
  );
});
