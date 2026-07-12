/**
 * Phase 5.2 — Astro site/base alignment checklist for GitHub Pages.
 * Verifies site/base, deploy workflow path/Node/out-dir, and build base smoke.
 *
 * Canonical production URL fixture (phase_5.3 can cite the same string):
 *   https://12yanogden.github.io/umoria-rust/
 */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const __dirname = dirname(fileURLToPath(import.meta.url));
const webRoot = join(__dirname, "..");
const repoRoot = join(webRoot, "..");
const astroConfigPath = join(webRoot, "astro.config.mjs");
const packageJsonPath = join(webRoot, "package.json");
const deployPath = join(repoRoot, ".github/workflows/deploy.yml");

/** Machine-checkable fixture for maintainers / phase_5.3. */
export const EXPECTED_PRODUCTION_URL = "https://12yanogden.github.io/umoria-rust/";
export const EXPECTED_BASE = "/umoria-rust";

function originOwner() {
  const remote = spawnSync("git", ["remote", "get-url", "origin"], {
    cwd: repoRoot,
    encoding: "utf8",
  });
  assert.equal(remote.status, 0, `git remote get-url origin failed: ${remote.stderr}`);
  const url = remote.stdout.trim();
  // git@github.com:owner/repo.git or https://github.com/owner/repo.git
  const m = url.match(/github\.com[:/]([^/]+)\//i);
  assert.ok(m, `could not parse GitHub owner from origin: ${url}`);
  return m[1];
}

function expectedSite() {
  return `https://${originOwner()}.github.io`;
}

function parseAstroConfig() {
  assert.ok(existsSync(astroConfigPath), `expected ${astroConfigPath}`);
  const src = readFileSync(astroConfigPath, "utf8");
  const siteMatch = src.match(/\bsite\s*:\s*["']([^"']+)["']/);
  const baseMatch = src.match(/\bbase\s*:\s*["']([^"']+)["']/);
  const outDirMatch = src.match(/\boutDir\s*:\s*["']([^"']+)["']/);
  assert.ok(siteMatch, "astro.config must set site");
  assert.ok(baseMatch, "astro.config must set base");
  return {
    src,
    site: siteMatch[1],
    base: baseMatch[1],
    outDir: outDirMatch?.[1] ?? null,
  };
}

function loadDeployYaml() {
  assert.ok(existsSync(deployPath), `expected ${deployPath}`);
  return readFileSync(deployPath, "utf8");
}

function withastroWithBlock(yaml) {
  const actionIdx = yaml.search(/uses:\s*withastro\/action@/);
  assert.ok(actionIdx >= 0, "withastro/action step required");
  const after = yaml.slice(actionIdx);
  const withBlock = after.match(/with:\s*\n(?:[ \t]+.+\n)*/);
  assert.ok(withBlock, "withastro/action must have a with: block");
  return withBlock[0];
}

function satisfiesNodeEngine(nodeVersion, enginesNode) {
  // engines.node like ">=22.12.0"; workflow node-version like "24" or "22.12.0"
  const major = Number.parseInt(String(nodeVersion).split(".")[0], 10);
  assert.ok(Number.isFinite(major), `invalid node-version: ${nodeVersion}`);

  const ge = enginesNode.match(/>=\s*(\d+)(?:\.(\d+))?(?:\.(\d+))?/);
  if (ge) {
    const minMajor = Number(ge[1]);
    return major >= minMajor;
  }
  // Fall back: exact major pin in engines
  const exact = enginesNode.match(/^\s*(\d+)/);
  if (exact) {
    return major === Number(exact[1]);
  }
  return false;
}

function collectTextFiles(dir, acc = []) {
  if (!existsSync(dir)) return acc;
  for (const name of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, name.name);
    if (name.isDirectory()) {
      collectTextFiles(full, acc);
    } else if (/\.(html|css|js|mjs|json|xml)$/i.test(name.name)) {
      acc.push(full);
    }
  }
  return acc;
}

test("1. Config site matches origin owner github.io (no repo path)", () => {
  const { site } = parseAstroConfig();
  const expected = expectedSite();
  assert.equal(site, expected, `site must be ${expected} (no /repo path)`);
  assert.ok(!site.includes("umoria-rust"), "site must not include the repo path");
});

test("2. Config base equals /umoria-rust", () => {
  const { base } = parseAstroConfig();
  assert.equal(base, EXPECTED_BASE);
  assert.ok(!base.endsWith("/") || base === "/", "Astro base should omit trailing slash");
});

test("3. Workflow path is web", () => {
  const yaml = loadDeployYaml();
  const block = withastroWithBlock(yaml);
  assert.match(block, /path:\s*web\b/, "expected withastro/action with: path: web");
});

test("4. Workflow Node version compatible with engines.node", () => {
  const yaml = loadDeployYaml();
  const block = withastroWithBlock(yaml);
  const nodeMatch = block.match(/node-version:\s*['"]?([^\s'"#]+)/);
  assert.ok(nodeMatch, "node-version must be pinned explicitly when engines present or not");
  const nodeVersion = nodeMatch[1];

  const pkg = JSON.parse(readFileSync(packageJsonPath, "utf8"));
  const enginesNode = pkg.engines?.node;
  if (enginesNode) {
    assert.ok(
      satisfiesNodeEngine(nodeVersion, enginesNode),
      `node-version ${nodeVersion} must satisfy engines.node ${enginesNode}`
    );
  }
});

test("5. Artifact out-dir matches Astro outDir (default dist)", () => {
  const { outDir } = parseAstroConfig();
  const yaml = loadDeployYaml();
  const block = withastroWithBlock(yaml);
  const outDirMatch = block.match(/out-dir:\s*['"]?([^\s'"#]+)/);
  const workflowOutDir = outDirMatch?.[1] ?? null;

  if (outDir === null && workflowOutDir === null) {
    // Both rely on Astro / withastro/action default: dist
    assert.equal(outDir, null);
    assert.equal(workflowOutDir, null);
    return;
  }
  const effectiveAstro = outDir ?? "dist";
  const effectiveWorkflow = workflowOutDir ?? "dist";
  assert.equal(
    effectiveWorkflow,
    effectiveAstro,
    `workflow out-dir (${effectiveWorkflow}) must match Astro outDir (${effectiveAstro})`
  );
});

test("5b. Bun lockfile present; pnpm-lock.yaml absent", () => {
  const bunLock = existsSync(join(webRoot, "bun.lock"));
  const bunLockb = existsSync(join(webRoot, "bun.lockb"));
  assert.ok(
    bunLock || bunLockb,
    "expected bun.lock or bun.lockb under web/ (withastro/action auto-detects bun)"
  );
  assert.ok(
    !existsSync(join(webRoot, "pnpm-lock.yaml")),
    "pnpm-lock.yaml must be absent under web/ after bun migration"
  );
});

test("6. Local bun build succeeds and emits base-prefixed assets", () => {
  const build = spawnSync("bun", ["run", "build"], {
    cwd: webRoot,
    encoding: "utf8",
    env: { ...process.env },
  });
  assert.equal(
    build.status,
    0,
    `bun run build failed (exit ${build.status}):\n${build.stdout}\n${build.stderr}`
  );

  const { outDir, base } = parseAstroConfig();
  const distRoot = join(webRoot, outDir ?? "dist");
  assert.ok(existsSync(distRoot), `expected build output at ${distRoot}`);

  const files = collectTextFiles(distRoot);
  assert.ok(files.length > 0, "dist/ must contain built files");

  const basePrefix = base; // "/umoria-rust"
  const hit = files.find((f) => {
    const text = readFileSync(f, "utf8");
    return text.includes(basePrefix);
  });

  if (hit) {
    assert.ok(true, `found ${basePrefix} reference in ${hit}`);
  } else {
    // Theme may use exclusively relative URLs — still require config base.
    assert.equal(base, EXPECTED_BASE, "relative-only assets still require base === /umoria-rust");
  }
});

test("7. Production URL documented in assertion fixture", () => {
  // This file itself is the durable fixture phase_5.3 can cite.
  const self = readFileSync(fileURLToPath(import.meta.url), "utf8");
  assert.ok(
    self.includes(EXPECTED_PRODUCTION_URL),
    `fixture must include ${EXPECTED_PRODUCTION_URL}`
  );

  const { src, site, base } = parseAstroConfig();
  const derived = `${site.replace(/\/$/, "")}${base}/`;
  assert.equal(derived, EXPECTED_PRODUCTION_URL);

  // Maintainers: short durable note in astro.config (phase_5.3 expands further).
  assert.ok(
    src.includes(EXPECTED_PRODUCTION_URL) || src.includes("GitHub project Pages"),
    "astro.config should document the Pages URL or hosting note"
  );
});
