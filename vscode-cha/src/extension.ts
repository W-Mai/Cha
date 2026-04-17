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

  // 2. Check extension-managed binary (preferred over system PATH)
  const stored = path.join(context.globalStorageUri.fsPath, "cha");
  if (fs.existsSync(stored) && commandExists(stored)) return stored;

  // 3. Fallback to system PATH
  if (commandExists("cha")) return "cha";

  // 4. Offer to download
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
    { location: vscode.ProgressLocation.Notification, title: "Downloading cha..." },
    async () => {
      try {
        const dir = context.globalStorageUri.fsPath;
        fs.mkdirSync(dir, { recursive: true });
        await downloadLatest(stored);
        fs.chmodSync(stored, 0o755);
        vscode.window.showInformationMessage("cha installed successfully.");
        return stored;
      } catch (e: any) {
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

async function downloadLatest(dest: string): Promise<void> {
  const arch = process.arch === "arm64" ? "aarch64" : "x86_64";
  const platform =
    process.platform === "darwin"
      ? `${arch}-apple-darwin`
      : `${arch}-unknown-linux-gnu`;

  // Get latest release tag
  const release = await fetchJson(
    "https://api.github.com/repos/W-Mai/Cha/releases/latest"
  );
  const tag = release.tag_name;
  const url = `https://github.com/W-Mai/Cha/releases/download/${tag}/cha-cli-${platform}.tar.xz`;

  // Download and extract
  const tarball = dest + ".tar.xz";
  await downloadFile(url, tarball);
  const dir = path.dirname(dest);
  cp.execSync(`tar xJf "${tarball}" -C "${dir}"`, { stdio: "ignore" });
  fs.unlinkSync(tarball);
  // cargo-dist extracts to cha-cli-<platform>/cha, move binary to dest
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

function downloadFile(url: string, dest: string): Promise<void> {
  return new Promise((resolve, reject) => {
    https.get(url, { headers: { "User-Agent": "vscode-cha" } }, (res) => {
      if (res.statusCode === 301 || res.statusCode === 302) {
        return downloadFile(res.headers.location!, dest).then(resolve, reject);
      }
      const file = fs.createWriteStream(dest);
      res.pipe(file);
      file.on("finish", () => { file.close(); resolve(); });
      file.on("error", reject);
    });
  });
}
