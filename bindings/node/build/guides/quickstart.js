"use strict";
// The first example on the README and the site: a reading off a wire, smoothed,
// signed, and packed for a metered link, with nothing plugged in.
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
// ANCHOR: example
const strict_1 = __importDefault(require("node:assert/strict"));
const codec_1 = require("@pamoja/core/codec");
const kit_1 = require("@pamoja/core/kit");
const security_1 = require("@pamoja/core/security");
const sensors_1 = require("@pamoja/core/sensors");
// The nine bytes a DS18B20 sends, CRC last; a bad CRC is a rejected read.
const scratchpad = Buffer.from([0x91, 0x01, 0x4b, 0x46, 0x7f, 0xff, 0x0c, 0x10, 0x00]);
scratchpad[8] = sensors_1.ds18b20.crc8(scratchpad.subarray(0, 8));
const celsius = sensors_1.ds18b20.parseScratchpad(scratchpad).microCelsius / 1e6;
strict_1.default.equal(celsius, 25.0625);
// Smooth the noise out of successive readings.
const smoother = new kit_1.Smoother(0.5);
smoother.update(celsius);
const smoothed = smoother.update(celsius + 1);
strict_1.default.ok(smoothed > celsius && smoothed < celsius + 1);
// Sign the reading so a gateway can prove which device sent it.
const device = security_1.DeviceIdentity.fromSeed(Buffer.alloc(32, 7));
const payload = smoothed.toFixed(2);
const signature = device.sign(payload);
strict_1.default.ok((0, security_1.verify)(device.publicKey(), payload, signature));
// Pack a batch of readings for a link where every byte costs money.
const samples = [2506, 2507, 2509, 2508, 2510];
const packed = (0, codec_1.packSamples)(samples);
strict_1.default.ok(packed.length < samples.length * 8);
strict_1.default.deepEqual((0, codec_1.unpackSamples)(packed), samples);
// ANCHOR_END: example
