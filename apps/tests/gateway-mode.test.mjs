import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { pathToFileURL } from "node:url";
import ts from "../node_modules/typescript/lib/typescript.js";

const appsRoot = path.resolve(import.meta.dirname, "..");
const sourcePath = path.join(appsRoot, "src", "lib", "gateway-mode.ts");

async function loadGatewayModeModule() {
  const source = await fs.readFile(sourcePath, "utf8");
  const compiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ES2022,
      target: ts.ScriptTarget.ES2022,
    },
    fileName: sourcePath,
  });

  const tempDir = await fs.mkdtemp(
    path.join(os.tmpdir(), "codexmanager-gateway-mode-")
  );
  const tempFile = path.join(tempDir, "gateway-mode.mjs");
  await fs.writeFile(tempFile, compiled.outputText, "utf8");
  return import(pathToFileURL(tempFile).href);
}

const gatewayMode = await loadGatewayModeModule();

test("normalizeGatewayMode 只接受 transparent，其余都回退到 enhanced", () => {
  assert.equal(gatewayMode.normalizeGatewayMode("enhanced"), "enhanced");
  assert.equal(gatewayMode.normalizeGatewayMode("ENHANCED"), "enhanced");
  assert.equal(gatewayMode.normalizeGatewayMode("transparent"), "transparent");
  assert.equal(gatewayMode.normalizeGatewayMode(""), "enhanced");
  assert.equal(gatewayMode.normalizeGatewayMode("other"), "enhanced");
});

test("toGatewayModeOverride 仅在强兼容模式下写入 override", () => {
  assert.equal(gatewayMode.toGatewayModeOverride("transparent"), "");
  assert.equal(gatewayMode.toGatewayModeOverride("enhanced"), "enhanced");
});
