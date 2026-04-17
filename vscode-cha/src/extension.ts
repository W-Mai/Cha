import * as vscode from "vscode";
import * as cp from "child_process";
import * as fs from "fs";
import * as path from "path";
import * as https from "https";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext) {
  const config = vscode.workspace.getConfiguration("cha");
  if (!config.get<boolean>("lsp.enabled", true)) return;

  const command = await ensureBinary(context, config.get<string>("path", "cha"));
  if (!command) return;

  const serverOptions: ServerOptions = { command, args: ["lsp"] };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "rust" },
      { scheme: "file", language: "typescript" },
      { scheme: "file", language: "python" },
      { scheme: "file", language: "go" },
      { scheme: "file", language: "c" },
      { scheme: "file", language: "cpp" },
    ],
  };

  client = new LanguageClient("cha", "Cha LSP", serverOptions, clientOptions);
  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}

async function ensureBinary(
  context: vscode.ExtensionContext,
  configured: string
): Promise<string | undefined> {
  // 1. User explicitly configured a path — use it
  if (configured !== "cha" && commandExists(configured)) return configured;

  // 2. Check extension-managed binary (preferred)
  const stored = path.join(context.globalStorageUri.fsPath, "cha");
  if (fs.existsSync(stored) && commandExists(stored)) return stored;

  // 3. Offer to download
  const choice = await vscode.window.showWarningMessage(
    "cha binary not found. Download from GitHub?",
    "Download",
    "Configure Path"
  );

  if (choice === "Configure Path") {
    vscode.commands.executeCommand(
      "workbench.action.openSettings",
      "cha.path"
    );
    return undefined;
  }
  if (choice !== "Download") return undefined;

  return downloadToStorage(context, stored);
}


async function downloadToStorage(
  context: vscode.ExtensionContext,
  stored: string
): Promise<string | undefined> {
  return vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "Downloading cha",
      cancellable: true,
    },
    async (progress, token) => {
      try {
        const dir = context.globalStorageUri.fsPath;
        fs.mkdirSync(dir, { recursive: true });
        await downloadLatest(stored, (pct, msg) => {
          progress.report({ increment: pct, message: msg });
        }, token);
        fs.chmodSync(stored, 0o755);
        vscode.window.showInformationMessage("cha installed successfully.");
        return stored;
      } catch (e: any) {
        if (token.isCancellationRequested) return undefined;
        vscode.window.showErrorMessage(`Failed to download cha: ${e.message}`);
        return undefined;
      }
    }
  );
}
function commandExists(cmd: string): boolean {
  try {
    cp.execSync(`"${cmd}" --version`, { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

type ProgressFn = (increment: number, message: string) => void;

async function downloadLatest(
  dest: string,
  progress: ProgressFn,
  token: vscode.CancellationToken
): Promise<void> {
  const arch = process.arch === "arm64" ? "aarch64" : "x86_64";
  const platform =
    process.platform === "darwin"
      ? `${arch}-apple-darwin`
      : `${arch}-unknown-linux-gnu`;

  progress(0, "fetching release info...");
  const release = await fetchJson(
    "https://api.github.com/repos/W-Mai/Cha/releases/latest"
  );
  const tag = release.tag_name;
  const url = `https://github.com/W-Mai/Cha/releases/download/${tag}/cha-cli-${platform}.tar.xz`;

  const tarball = dest + ".tar.xz";
  await downloadFileWithProgress(url, tarball, (pct) => {
    progress(pct, `downloading ${tag}... ${pct.toFixed(0)}%`);
  }, token);

  progress(0, "extracting...");
  const dir = path.dirname(dest);
  cp.execSync(`tar xJf "${tarball}" -C "${dir}"`, { stdio: "ignore" });
  fs.unlinkSync(tarball);
  const extracted = path.join(dir, `cha-cli-${platform}`, "cha");
  if (fs.existsSync(extracted)) {
    fs.renameSync(extracted, dest);
    fs.rmSync(path.join(dir, `cha-cli-${platform}`), { recursive: true, force: true });
  }
}

function fetchJson(url: string): Promise<any> {
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

function downloadFileWithProgress(
  url: string,
  dest: string,
  onProgress: (pct: number) => void,
  token: vscode.CancellationToken
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
          const increment = pct - lastPct;
          if (increment >= 1) {
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
