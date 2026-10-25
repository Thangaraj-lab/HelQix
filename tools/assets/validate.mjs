// Khronos glTF-Validator gate (Apache-2.0, official JS build).
// Usage: node validate.mjs <file.glb|file.gltf>
// Prints a JSON summary; exit 0 only if the validator reports zero errors.
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import validator from "gltf-validator";

const file = process.argv[2];
if (!file) { console.error("usage: node validate.mjs <file>"); process.exit(1); }
const bytes = new Uint8Array(await readFile(file));
const report = await validator.validateBytes(bytes, {
  uri: file,
  maxIssues: 200,
  // External resources (for .gltf with separate files) are read from the same directory.
  externalResourceFunction: (uri) => readFile(join(dirname(file), decodeURIComponent(uri))).then((b) => new Uint8Array(b)),
});
const summary = {
  validatorVersion: report.validatorVersion,
  errors: report.issues.numErrors,
  warnings: report.issues.numWarnings,
  infos: report.issues.numInfos,
  hints: report.issues.numHints,
  messages: report.issues.messages.filter((m) => m.severity <= 1).slice(0, 20).map((m) => `${m.severity === 0 ? "ERROR" : "WARNING"} ${m.code} ${m.pointer ?? ""} ${m.message}`),
  info: { generator: report.info?.generator, extensionsUsed: report.info?.extensionsUsed ?? [], textures: report.info?.resources?.filter((r) => r.storage !== undefined && r.pointer?.startsWith("/images")).map((r) => r.mimeType) ?? [] },
};
console.log(JSON.stringify(summary));
process.exit(summary.errors === 0 ? 0 : 2);
