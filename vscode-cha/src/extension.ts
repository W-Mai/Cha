import * as vscode from "vscode";
import * as cp from "child_process";
import * as fs from "fs";
import * as path from "path";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";
import { downloadLatest } from "./download";

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
  const bin = process.platform === "win32" ? "cha.exe" : "cha";
  const stored = path.join(context.globalStorageUri.fsPath, bin);
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
