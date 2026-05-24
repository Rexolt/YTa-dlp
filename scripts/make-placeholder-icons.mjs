#!/usr/bin/env node
// Generates minimal valid PNG icons (solid magenta) so Tauri's
// `generate_context!()` macro doesn't panic during dev compilation.
// Replace with real artwork via `pnpm tauri icon path/to/icon.png` later.

import { mkdirSync, writeFileSync, existsSync } from 'node:fs';
import { deflateSync } from 'node:zlib';
import { resolve, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const OUT  = join(ROOT, 'src-tauri', 'icons');

function crc32(buf) {
  let c, table = [];
  for (let n = 0; n < 256; n++) {
    c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++) crc = (crc >>> 8) ^ table[(crc ^ buf[i]) & 0xff];
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(name, data) {
  const len = Buffer.alloc(4);  len.writeUInt32BE(data.length);
  const nameBuf = Buffer.from(name, 'ascii');
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE(crc32(Buffer.concat([nameBuf, data])));
  return Buffer.concat([len, nameBuf, data, crcBuf]);
}

function makePng(path, w, h, [r, g, b, a] = [168, 85, 247, 255]) {
  const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8;  // bit depth
  ihdr[9] = 6;  // color type: RGBA (required by Tauri)
  ihdr[10] = 0; // compression
  ihdr[11] = 0; // filter
  ihdr[12] = 0; // interlace

  // Each scanline = filter byte (0) + RGBA quads
  const row = Buffer.alloc(1 + w * 4);
  row[0] = 0;
  for (let x = 0; x < w; x++) {
    row[1 + x * 4]     = r;
    row[1 + x * 4 + 1] = g;
    row[1 + x * 4 + 2] = b;
    row[1 + x * 4 + 3] = a;
  }
  const raw = Buffer.alloc(row.length * h);
  for (let y = 0; y < h; y++) row.copy(raw, y * row.length);
  const idat = deflateSync(raw, { level: 9 });

  const png = Buffer.concat([
    sig,
    chunk('IHDR', ihdr),
    chunk('IDAT', idat),
    chunk('IEND', Buffer.alloc(0))
  ]);
  writeFileSync(path, png);
  console.log(`✓ ${path} (${w}x${h})`);
}

mkdirSync(OUT, { recursive: true });
const files = [
  ['32x32.png', 32, 32],
  ['128x128.png', 128, 128],
  ['128x128@2x.png', 256, 256]
];

const force = process.argv.includes('--force');
for (const [name, w, h] of files) {
  const p = join(OUT, name);
  if (existsSync(p) && !force) { console.log(`· skip ${name} (exists)`); continue; }
  makePng(p, w, h);
}
console.log('Done. Replace with real icons via `pnpm tauri icon <source.png>`.');
