/**
 * Download logic for cha binary — shared between extension and tests.
 * No vscode dependency so it can be tested standalone.
 */
import * as https from "https";
import * as fs from "fs";
import * as cp from "child_process";
import * as path from "path";

export type ProgressFn = (pct: number, message: string) => void;

export interface Cancellable {
  onCancellationRequested(cb: () => void): void;
}

export function getPlatformTriple(): { triple: string; isWin: boolean } {
  const arch = process.arch === "arm64" ? "aarch64" : "x86_64";
  const isWin = process.platform === "win32";
  const triple = process.platform === "darwin"
    ? `${arch}-apple-darwin`
    : isWin
      ? `${arch}-pc-windows-msvc`
      : `${arch}-unknown-linux-gnu`;
  return { triple, isWin };
}

export async function downloadLatest(
  dest: string,
  progress: ProgressFn,
  token: Cancellable,
): Promise<void> {
  const { triple: platform, isWin } = getPlatformTriple();

  progress(0, "fetching release info...");
  const release = await fetchJson(
    "https://api.github.com/repos/W-Mai/Cha/releases/latest"
  );
  const tag = release.tag_name;

  const dir = path.dirname(dest);
  fs.mkdirSync(dir, { recursive: true });

  if (isWin) {
    const url = `https://github.com/W-Mai/Cha/releases/download/${tag}/cha-cli-${platform}.zip`;
    const zipPath = dest + ".zip";
    await downloadFileWithProgress(url, zipPath, (pct) => {
      progress(pct, `downloading ${tag}... ${pct.toFixed(0)}%`);
    }, token);
    progress(0, "extracting...");
    cp.execSync(`powershell -Command "Expand-Archive -Force '${zipPath}' '${dir}'"`, { stdio: "ignore" });
    fs.unlinkSync(zipPath);
    const extracted = path.join(dir, `cha-cli-${platform}`, "cha.exe");
    if (fs.existsSync(extracted)) {
      fs.renameSync(extracted, dest + ".exe");
      fs.rmSync(path.join(dir, `cha-cli-${platform}`), { recursive: true, force: true });
    }
  } else {
    const url = `https://github.com/W-Mai/Cha/releases/download/${tag}/cha-cli-${platform}.tar.xz`;
    const tarball = dest + ".tar.xz";
    await downloadFileWithProgress(url, tarball, (pct) => {
      progress(pct, `downloading ${tag}... ${pct.toFixed(0)}%`);
    }, token);
    progress(0, "extracting...");
    cp.execSync(`tar xJf "${tarball}" -C "${dir}"`, { stdio: "ignore" });
    fs.unlinkSync(tarball);
    const extracted = path.join(dir, `cha-cli-${platform}`, "cha");
    if (fs.existsSync(extracted)) {
      fs.renameSync(extracted, dest);
      fs.rmSync(path.join(dir, `cha-cli-${platform}`), { recursive: true, force: true });
    }
  }
}

export function fetchJson(url: string): Promise<any> {
  return new Promise((resolve, reject) => {
    https.get(url, { headers: { "User-Agent": "vscode-cha" } }, (res) => {
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

export function downloadFileWithProgress(
  url: string,
  dest: string,
  onProgress: (pct: number) => void,
  token: Cancellable,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const req = https.get(url, { headers: { "User-Agent": "vscode-cha" } }, (res) => {
      if (res.statusCode === 301 || res.statusCode === 302) {
        return downloadFileWithProgress(res.headers.location!, dest, onProgress, token).then(resolve, reject);
      }
      const total = parseInt(res.headers["content-length"] || "0", 10);
      let downloaded = 0;
      let lastPct = 0;
      const file = fs.createWriteStream(dest);
      res.on("data", (chunk: Buffer) => {
        downloaded += chunk.length;
        if (total > 0) {
          const pct = (downloaded / total) * 100;
          if (pct - lastPct >= 1) {
            onProgress(pct);
            lastPct = pct;
          }
        }
      });
      res.pipe(file);
      file.on("finish", () => { file.close(); resolve(); });
      file.on("error", reject);
    });
    token.onCancellationRequested(() => { req.destroy(); reject(new Error("cancelled")); });
  });
}
