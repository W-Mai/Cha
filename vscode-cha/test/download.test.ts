/**
 * E2E test for the VS Code extension download flow.
 * Verifies that the binary can be downloaded and extracted for all platforms.
 *
 * Usage: npx tsx test/download.test.ts [--platform darwin|linux|win32] [--arch x64|arm64]
 */

import * as https from "https";
import * as fs from "fs";
import * as cp from "child_process";
import * as path from "path";
import * as os from "os";

const REPO = "W-Mai/Cha";

function fetchJson(url: string): Promise<any> {
  return new Promise((resolve, reject) => {
    https.get(url, { headers: { "User-Agent": "cha-e2e-test" } }, (res) => {
      if (res.statusCode === 301 || res.statusCode === 302) {
        return fetchJson(res.headers.location!).then(resolve, reject);
      }
      let data = "";
      res.on("data", (chunk) => (data += chunk));
      res.on("end", () => resolve(JSON.parse(data)));
      res.on("error", reject);
    });
  });
}

function downloadFile(url: string, dest: string): Promise<void> {
  return new Promise((resolve, reject) => {
    https.get(url, { headers: { "User-Agent": "cha-e2e-test" } }, (res) => {
      if (res.statusCode === 301 || res.statusCode === 302) {
        return downloadFile(res.headers.location!, dest).then(resolve, reject);
      }
      const file = fs.createWriteStream(dest);
      res.pipe(file);
      file.on("finish", () => {
        file.close();
        resolve();
      });
      file.on("error", reject);
    });
  });
}

interface PlatformConfig {
  triple: string;
  ext: string;
  extractCmd: (archive: string, dir: string) => string;
  binaryName: string;
}

function getPlatformConfig(
  platform: string,
  arch: string
): PlatformConfig {
  const cpuArch = arch === "arm64" ? "aarch64" : "x86_64";
  if (platform === "win32") {
    return {
      triple: `${cpuArch}-pc-windows-msvc`,
      ext: ".zip",
      extractCmd: (archive, dir) =>
        `powershell -Command "Expand-Archive -Force '${archive}' '${dir}'"`,
      binaryName: "cha.exe",
    };
  }
  const os = platform === "darwin" ? "apple-darwin" : "unknown-linux-gnu";
  return {
    triple: `${cpuArch}-${os}`,
    ext: ".tar.xz",
    extractCmd: (archive, dir) => `tar xJf "${archive}" -C "${dir}"`,
    binaryName: "cha",
  };
}

async function testDownload(platform: string, arch: string) {
  const cfg = getPlatformConfig(platform, arch);
  const label = `${platform}/${arch} (${cfg.triple})`;
  console.log(`\n🧪 Testing download for ${label}...`);

  // 1. Fetch latest release
  console.log("  [1/4] Fetching release info...");
  const release = await fetchJson(
    `https://api.github.com/repos/${REPO}/releases/latest`
  );
  const tag = release.tag_name;
  console.log(`  → tag: ${tag}`);

  // 2. Check asset exists
  const assetName = `cha-cli-${cfg.triple}${cfg.ext}`;
  const asset = release.assets.find((a: any) => a.name === assetName);
  if (!asset) {
    const available = release.assets.map((a: any) => a.name).join(", ");
    throw new Error(
      `Asset ${assetName} not found in release ${tag}. Available: ${available}`
    );
  }
  console.log(`  [2/4] Asset found: ${assetName} (${(asset.size / 1024 / 1024).toFixed(1)} MB)`);

  // 3. Download
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "cha-e2e-"));
  const archivePath = path.join(tmpDir, assetName);
  const downloadUrl = `https://github.com/${REPO}/releases/download/${tag}/${assetName}`;
  console.log(`  [3/4] Downloading...`);
  await downloadFile(downloadUrl, archivePath);
  const dlSize = fs.statSync(archivePath).size;
  console.log(`  → downloaded ${(dlSize / 1024 / 1024).toFixed(1)} MB`);

  // 4. Extract (only on matching platform)
  if (process.platform === platform) {
    console.log(`  [4/4] Extracting...`);
    cp.execSync(cfg.extractCmd(archivePath, tmpDir), { stdio: "ignore" });
    const binaryPath = path.join(tmpDir, `cha-cli-${cfg.triple}`, cfg.binaryName);
    if (!fs.existsSync(binaryPath)) {
      throw new Error(`Binary not found at ${binaryPath}`);
    }
    fs.chmodSync(binaryPath, 0o755);
    const version = cp.execSync(`"${binaryPath}" --version`, { encoding: "utf8" }).trim();
    console.log(`  → binary works: ${version}`);
  } else {
    console.log(`  [4/4] Skipping extraction (cross-platform, ${process.platform} ≠ ${platform})`);
  }

  // Cleanup
  fs.rmSync(tmpDir, { recursive: true, force: true });
  console.log(`  ✅ ${label} passed`);
}

async function main() {
  const args = process.argv.slice(2);
  const platformIdx = args.indexOf("--platform");
  const archIdx = args.indexOf("--arch");

  if (platformIdx !== -1) {
    // Test specific platform
    const platform = args[platformIdx + 1];
    const arch = archIdx !== -1 ? args[archIdx + 1] : process.arch;
    await testDownload(platform, arch);
  } else {
    // Test all platforms
    const platforms = [
      ["darwin", "arm64"],
      ["darwin", "x64"],
      ["linux", "x64"],
      ["linux", "arm64"],
      ["win32", "x64"],
    ];
    let passed = 0;
    let failed = 0;
    for (const [p, a] of platforms) {
      try {
        await testDownload(p, a);
        passed++;
      } catch (e: any) {
        console.error(`  ❌ ${p}/${a} failed: ${e.message}`);
        failed++;
      }
    }
    console.log(`\n${passed} passed, ${failed} failed`);
    if (failed > 0) process.exit(1);
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
