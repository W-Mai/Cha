/**
 * E2E test for the VS Code extension download flow.
 * Imports the actual download logic from src/download.ts — same code the extension uses.
 *
 * Usage: npx tsx test/download.test.ts
 */
import * as fs from "fs";
import * as cp from "child_process";
import * as path from "path";
import * as os from "os";
import { downloadLatest, fetchJson, Cancellable } from "../src/download";

const REPO = "W-Mai/Cha";

const noop: Cancellable = { onCancellationRequested: () => {} };

async function testCurrentPlatform() {
  console.log(`\n🧪 Testing downloadLatest on ${process.platform}/${process.arch}...`);

  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "cha-e2e-"));
  const dest = path.join(tmpDir, "cha");

  await downloadLatest(dest, (pct, msg) => {
    // silent
  }, noop);

  const bin = process.platform === "win32" ? dest + ".exe" : dest;
  if (!fs.existsSync(bin)) {
    throw new Error(`Binary not found at ${bin}`);
  }
  const version = cp.execSync(`"${bin}" --version`, { encoding: "utf8" }).trim();
  console.log(`  → binary works: ${version}`);

  fs.rmSync(tmpDir, { recursive: true, force: true });
  console.log(`  ✅ current platform passed`);
}

async function testAllAssetsExist() {
  console.log(`\n🧪 Verifying all platform assets exist in latest release...`);

  const release = await fetchJson(
    `https://api.github.com/repos/${REPO}/releases/latest`
  );
  const tag = release.tag_name;
  const assetNames = release.assets.map((a: any) => a.name);
  console.log(`  → tag: ${tag}, ${assetNames.length} assets`);

  const expected = [
    "cha-cli-aarch64-apple-darwin.tar.xz",
    "cha-cli-x86_64-apple-darwin.tar.xz",
    "cha-cli-x86_64-unknown-linux-gnu.tar.xz",
    "cha-cli-aarch64-unknown-linux-gnu.tar.xz",
    "cha-cli-x86_64-pc-windows-msvc.zip",
  ];

  let ok = 0;
  for (const name of expected) {
    if (assetNames.includes(name)) {
      console.log(`  ✓ ${name}`);
      ok++;
    } else {
      console.error(`  ✗ ${name} MISSING`);
    }
  }
  if (ok < expected.length) {
    throw new Error(`${expected.length - ok} assets missing`);
  }
  console.log(`  ✅ all ${ok} assets present`);
}

async function main() {
  await testAllAssetsExist();
  await testCurrentPlatform();
  console.log("\n✅ All download tests passed");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
