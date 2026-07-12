/**
 * Phase 5.1 — structural assertions for the GitHub Pages deploy workflow.
 * Does not require a site build; validates `.github/workflows/deploy.yml` only.
 */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { platform, arch } from "node:os";
import { fileURLToPath } from "node:url";
import test from "node:test";

const __dirname = dirname(fileURLToPath(import.meta.url));
const webRoot = join(__dirname, "..");
const repoRoot = join(webRoot, "..");
const workflowsDir = join(repoRoot, ".github/workflows");
const deployPath = join(workflowsDir, "deploy.yml");

const ACTIONLINT_VERSION = "1.7.7";

function loadDeployYaml() {
  assert.ok(existsSync(deployPath), `expected ${deployPath} to exist`);
  return readFileSync(deployPath, "utf8");
}

function actionlintAssetName() {
  const os = platform();
  const cpu = arch();
  if (os === "darwin" && cpu === "arm64") return `actionlint_${ACTIONLINT_VERSION}_darwin_arm64.tar.gz`;
  if (os === "darwin" && cpu === "x64") return `actionlint_${ACTIONLINT_VERSION}_darwin_amd64.tar.gz`;
  if (os === "linux" && cpu === "arm64") return `actionlint_${ACTIONLINT_VERSION}_linux_arm64.tar.gz`;
  if (os === "linux" && cpu === "x64") return `actionlint_${ACTIONLINT_VERSION}_linux_amd64.tar.gz`;
  throw new Error(`unsupported platform for actionlint bootstrap: ${os}/${cpu}`);
}

function ensureActionlint() {
  const fromPath = spawnSync("actionlint", ["-version"], { encoding: "utf8" });
  if (fromPath.status === 0) {
    return "actionlint";
  }

  const cacheDir = join(webRoot, ".cache/actionlint");
  const bin = join(cacheDir, "actionlint");
  if (existsSync(bin)) {
    return bin;
  }

  mkdirSync(cacheDir, { recursive: true });
  const asset = actionlintAssetName();
  const url = `https://github.com/rhysd/actionlint/releases/download/v${ACTIONLINT_VERSION}/${asset}`;
  const archive = join(cacheDir, asset);
  const curl = spawnSync("curl", ["-fsSL", "-o", archive, url], { encoding: "utf8" });
  assert.equal(curl.status, 0, `failed to download actionlint: ${curl.stderr || curl.stdout}`);
  const tar = spawnSync("tar", ["-xzf", archive, "-C", cacheDir, "actionlint"], { encoding: "utf8" });
  assert.equal(tar.status, 0, `failed to extract actionlint: ${tar.stderr || tar.stdout}`);
  assert.ok(existsSync(bin), `actionlint binary missing after extract: ${bin}`);
  return bin;
}

test("1. Workflow file exists", () => {
  assert.ok(existsSync(deployPath), "expected .github/workflows/deploy.yml");
});

test("2. Builds web/ via withastro/action path: web", () => {
  const yaml = loadDeployYaml();
  assert.match(yaml, /withastro\/action@/, "expected withastro/action");
  assert.match(yaml, /path:\s*web\b/, "expected withastro/action with: path: web");
});

test("3. Permissions correct", () => {
  const yaml = loadDeployYaml();
  const permBlock = yaml.match(/permissions:\s*\n(?:[ \t]+.+\n)+/);
  assert.ok(permBlock, "expected top-level permissions: block");
  const block = permBlock[0];
  assert.match(block, /contents:\s*read/, "permissions must include contents: read");
  assert.match(block, /pages:\s*write/, "permissions must include pages: write");
  assert.match(block, /id-token:\s*write/, "permissions must include id-token: write");
});

test("4. Concurrency correct", () => {
  const yaml = loadDeployYaml();
  assert.match(yaml, /concurrency:\s*\n/, "expected concurrency: block");
  assert.match(yaml, /group:\s*["']?pages["']?/, "concurrency group must be pages");
  assert.match(yaml, /cancel-in-progress:\s*false/, "cancel-in-progress must be false");
});

test("5. Deploy job wiring", () => {
  const yaml = loadDeployYaml();
  assert.match(yaml, /\bdeploy\s*:/, "expected deploy job");
  assert.match(yaml, /needs:\s*build\b/, "deploy job must needs: build");
  assert.match(yaml, /actions\/deploy-pages@/, "expected actions/deploy-pages");
  assert.match(
    yaml,
    /environment:\s*\n\s+name:\s*github-pages/,
    "environment.name must be github-pages"
  );
});

test("6. actionlint / schema validation", () => {
  assert.ok(existsSync(deployPath), "deploy.yml required before actionlint");
  const bin = ensureActionlint();
  const result = spawnSync(bin, [deployPath], { encoding: "utf8" });
  assert.equal(
    result.status,
    0,
    `actionlint failed (exit ${result.status}):\n${result.stdout}${result.stderr}`
  );
});

test("7. Node pin documented under withastro/action", () => {
  const yaml = loadDeployYaml();
  const actionIdx = yaml.search(/uses:\s*withastro\/action@/);
  assert.ok(actionIdx >= 0, "withastro/action step required");
  const after = yaml.slice(actionIdx);
  const withBlock = after.match(/with:\s*\n(?:[ \t]+.+\n)+/);
  assert.ok(withBlock, "withastro/action must have a with: block");
  assert.match(withBlock[0], /node-version:\s*['"]?\d+/, "node-version must be pinned explicitly");
});

test("8. Package manager left to lockfile auto-detect (bun expected)", () => {
  const yaml = loadDeployYaml();
  const block = (() => {
    const actionIdx = yaml.search(/uses:\s*withastro\/action@/);
    assert.ok(actionIdx >= 0, "withastro/action step required");
    const after = yaml.slice(actionIdx);
    const withBlock = after.match(/with:\s*\n(?:[ \t]+.+\n)*/);
    assert.ok(withBlock, "withastro/action must have a with: block");
    return withBlock[0];
  })();
  // Prefer auto-detect from bun.lock / bun.lockb; do not pin package-manager: pnpm.
  assert.doesNotMatch(
    block,
    /package-manager:\s*['"]?pnpm\b/,
    "must not force package-manager: pnpm"
  );
  // Either omit package-manager (preferred) or explicitly set bun.
  const pmMatch = block.match(/package-manager:\s*['"]?([^\s'"#]+)/);
  if (pmMatch) {
    assert.equal(pmMatch[1], "bun", "if package-manager is set, it must be bun");
  } else {
    assert.match(
      yaml,
      /bun\.lock|auto-detect|lockfile/i,
      "when package-manager is omitted, deploy.yml should note bun lockfile auto-detect"
    );
  }

  const bunLock = existsSync(join(webRoot, "bun.lock"));
  const bunLockb = existsSync(join(webRoot, "bun.lockb"));
  assert.ok(
    bunLock || bunLockb,
    "expected bun.lock or bun.lockb under web/ for withastro/action auto-detect"
  );
  assert.ok(
    !existsSync(join(webRoot, "pnpm-lock.yaml")),
    "pnpm-lock.yaml must be absent under web/"
  );
});

test("exactly one Pages deploy workflow under .github/workflows", () => {
  assert.ok(existsSync(workflowsDir), ".github/workflows must exist");
  const pagesish = readdirSync(workflowsDir).filter((name) =>
    /^(deploy|pages|github-pages)\.ya?ml$/i.test(name)
  );
  assert.deepEqual(pagesish, ["deploy.yml"], `expected only deploy.yml, found: ${pagesish.join(", ")}`);
});
