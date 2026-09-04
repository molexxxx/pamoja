// Runs every compiled guide example as its own process and stops at the first
// failure. Each example is spliced into a page of the documentation site by
// `cargo xtask docs`, so every TypeScript example the site shows is code that ran.
const { readdirSync } = require("node:fs");
const { join } = require("node:path");
const { spawnSync } = require("node:child_process");

const dir = join(__dirname, "..", "build", "guides");
const guides = readdirSync(dir)
  .filter((name) => name.endsWith(".js"))
  .sort();

if (guides.length === 0) {
  console.error(`no compiled guides under ${dir}; run tsc -p guides/tsconfig.json first`);
  process.exit(1);
}

for (const guide of guides) {
  const result = spawnSync(process.execPath, [join(dir, guide)], { stdio: "inherit" });
  if (result.status !== 0) {
    console.error(`guide ${guide} failed`);
    process.exit(result.status ?? 1);
  }
  console.log(`guide ${guide} ok`);
}
