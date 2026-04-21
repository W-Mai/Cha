import * as assert from "assert";
import * as vscode from "vscode";
import * as sinon from "sinon";

suite("VS Code Extension E2E", () => {
  let stub: sinon.SinonStub;

  setup(() => {
    // Simulate user clicking "Download" when prompted
    stub = sinon.stub(vscode.window, "showWarningMessage").resolves(
      "Download" as any
    );
  });

  teardown(() => {
    stub.restore();
  });

  test("extension activates and cha binary is available", async () => {
    // Wait for our extension to activate
    const ext = vscode.extensions.getExtension("BenignX.vscode-cha");
    assert.ok(ext, "Extension not found");

    if (!ext.isActive) {
      await ext.activate();
    }
    assert.ok(ext.isActive, "Extension failed to activate");

    // Give it time to download binary if needed
    await new Promise((r) => setTimeout(r, 30_000));

    // Write a temp file (LSP only handles file: scheme, not untitled:)
    const tmpDir = require("os").tmpdir();
    const tmpFile = require("path").join(tmpDir, "cha-e2e-test.ts");
    require("fs").writeFileSync(
      tmpFile,
      `function veryLongFunction() {\n${"  const x = 1;\n".repeat(60)}}\n`
    );
    const doc = await vscode.workspace.openTextDocument(
      vscode.Uri.file(tmpFile)
    );
    await vscode.window.showTextDocument(doc);

    // Wait for LSP diagnostics
    await new Promise((r) => setTimeout(r, 15_000));
    const diags = vscode.languages.getDiagnostics(doc.uri);
    const chaFindings = diags.filter((d) => d.source === "cha");

    // Cleanup
    require("fs").unlinkSync(tmpFile);

    console.log(`  → ${chaFindings.length} cha diagnostics found`);
    assert.ok(
      chaFindings.length > 0,
      "Expected cha diagnostics for a 60-line function (long_method)"
    );
  });
});
