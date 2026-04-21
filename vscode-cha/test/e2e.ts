/**
 * E2E: launch real VS Code, install the extension, verify it downloads cha and starts LSP.
 */
import * as path from "path";
import { runTests, downloadAndUnzipVSCode } from "@vscode/test-electron";

async function main() {
  const extensionDevelopmentPath = path.resolve(__dirname, "..", "..");
  const extensionTestsPath = path.resolve(__dirname, "suite", "index");

  const vscodeExecutablePath = await downloadAndUnzipVSCode("stable");

  await runTests({
    vscodeExecutablePath,
    extensionDevelopmentPath,
    extensionTestsPath,
    launchArgs: [
      "--disable-gpu",
      path.resolve(__dirname, "..", ".."), // open project root as workspace
    ],
  });
}

main().catch((err) => {
  console.error("Failed to run e2e tests:", err);
  process.exit(1);
});
