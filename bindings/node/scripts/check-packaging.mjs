// Fails when a workspace package would publish without the code it promises.
//
// Each facade names `dist/index.js` as its entry and lists `dist/` in `files`. npm drops a
// `files` entry that is not on disk rather than failing, so publishing without building
// first yields a tarball holding only package.json, the README and the licence, which
// installs cleanly and throws MODULE_NOT_FOUND on the first require.

import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = dirname(dirname(fileURLToPath(import.meta.url)))

/**
 * Expands the root manifest's workspace globs to the directories they name.
 *
 * @returns {string[]} each workspace package directory, absolute.
 */
function workspaceDirectories() {
  const manifest = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'))
  const directories = []

  for (const pattern of manifest.workspaces ?? []) {
    if (!pattern.endsWith('/*')) {
      directories.push(join(root, pattern))
      continue
    }
    const parent = join(root, pattern.slice(0, -2))
    for (const entry of readdirSync(parent, { withFileTypes: true })) {
      if (entry.isDirectory() && existsSync(join(parent, entry.name, 'package.json'))) {
        directories.push(join(parent, entry.name))
      }
    }
  }

  return directories
}

const missing = []
const checked = []

for (const directory of workspaceDirectories()) {
  const manifest = JSON.parse(readFileSync(join(directory, 'package.json'), 'utf8'))
  if (manifest.private) continue

  for (const field of ['main', 'types']) {
    const declared = manifest[field]
    if (!declared) continue
    checked.push(`${manifest.name}#${field}`)
    if (!existsSync(join(directory, declared))) {
      missing.push(`${manifest.name} declares ${field} "${declared}" and it is not on disk`)
    }
  }
}

if (missing.length > 0) {
  console.error('These packages would publish without the code they promise:\n')
  for (const line of missing) console.error(`  ${line}`)
  console.error('\nRun `npm run build` before packing or publishing.')
  process.exit(1)
}

console.log(`packaging ok: ${checked.length} declared entry points all present`)
