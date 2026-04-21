import * as assert from "assert";
import * as vscode from "vscode";

suite("VS Code Extension E2E", () => {
  test("extension activates and cha binary is available", async () => {
    // Wait for our extension to activate
    const ext = vscode.extensions.getExtension("benignx.vscode-cha");
    assert.ok(ext, "Extension not found");

    if (!ext.isActive) {
      await ext.activate();
    }
    assert.ok(ext.isActive, "Extension failed to activate");

    // Give it time to download binary if needed
    await new Promise((r) => setTimeout(r, 30_000));

    // Verify the LSP client started by checking if cha language features are registered
    // Open a test file and check diagnostics work
    const doc = await vscode.workspace.openTextDocument({
      language: "typescript",
      content: `function veryLongFunction() {\n${"  const x = 1;\n".repeat(60)}}`,
    });
    await vscode.window.showTextDocument(doc);

    // Wait for diagnostics
    await new Promise((r) => setTimeout(r, 10_000));
    const diags = vscode.languages.getDiagnostics(doc.uri);
    const chaFindings = diags.filter((d) => d.source === "cha");

    console.log(`  → ${chaFindings.length} cha diagnostics found`);
    assert.ok(
      chaFindings.length > 0,
      "Expected cha diagnostics for a 60-line function (long_method)"
    );
  });
});
