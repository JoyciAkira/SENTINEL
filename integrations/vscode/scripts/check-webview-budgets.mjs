#!/usr/bin/env node
import { promises as fs } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const assetsDir = path.resolve(__dirname, "../out/webview/assets");

function readBudget(name, fallback) {
  const raw = process.env[name];
  if (!raw) return fallback;
  const parsed = Number(raw);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    throw new Error(`Invalid numeric budget ${name}=${raw}`);
  }
  return parsed;
}

function toKb(bytes) {
  return bytes / 1024;
}

function formatKb(bytes) {
  return `${toKb(bytes).toFixed(2)} KB`;
}

async function loadAssetFiles() {
  const entries = await fs.readdir(assetsDir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    if (!entry.isFile()) continue;
    const fullPath = path.join(assetsDir, entry.name);
    const stat = await fs.stat(fullPath);
    files.push({
      name: entry.name,
      bytes: stat.size,
    });
  }
  return files;
}

async function main() {
  const budgets = {
    maxIndexJsKb: readBudget("MAX_INDEX_JS_KB", 95),
    maxLargestVendorJsKb: readBudget("MAX_LARGEST_VENDOR_JS_KB", 420),
    maxTotalJsKb: readBudget("MAX_TOTAL_JS_KB", 650),
    maxIndexCssKb: readBudget("MAX_INDEX_CSS_KB", 90),
    maxTotalCssKb: readBudget("MAX_TOTAL_CSS_KB", 120),
  };

  let files;
  try {
    files = await loadAssetFiles();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`[budget] unable to read assets in ${assetsDir}: ${message}`);
    console.error("[budget] run 'npm run build:webview' before this check.");
    process.exit(1);
  }

  const jsFiles = files.filter((file) => file.name.endsWith(".js"));
  const cssFiles = files.filter((file) => file.name.endsWith(".css"));

  const indexJs = jsFiles.find((file) => /^index-.*\.js$/.test(file.name));
  const indexCss = cssFiles.find((file) => /^index-.*\.css$/.test(file.name));

  const vendorJsFiles = jsFiles.filter((file) => /^vendor-.*\.js$/.test(file.name));
  const largestVendorJs = vendorJsFiles.reduce(
    (max, file) => (file.bytes > max.bytes ? file : max),
    { name: "<none>", bytes: 0 },
  );

  const totalJsBytes = jsFiles.reduce((sum, file) => sum + file.bytes, 0);
  const totalCssBytes = cssFiles.reduce((sum, file) => sum + file.bytes, 0);

  const checks = [];
  if (!indexJs) {
    checks.push("Missing index-*.js bundle.");
  } else if (toKb(indexJs.bytes) > budgets.maxIndexJsKb) {
    checks.push(
      `index JS too large: ${formatKb(indexJs.bytes)} > ${budgets.maxIndexJsKb.toFixed(2)} KB`,
    );
  }

  if (!indexCss) {
    checks.push("Missing index-*.css bundle.");
  } else if (toKb(indexCss.bytes) > budgets.maxIndexCssKb) {
    checks.push(
      `index CSS too large: ${formatKb(indexCss.bytes)} > ${budgets.maxIndexCssKb.toFixed(2)} KB`,
    );
  }

  if (toKb(largestVendorJs.bytes) > budgets.maxLargestVendorJsKb) {
    checks.push(
      `largest vendor JS too large (${largestVendorJs.name}): ${formatKb(largestVendorJs.bytes)} > ${budgets.maxLargestVendorJsKb.toFixed(2)} KB`,
    );
  }

  if (toKb(totalJsBytes) > budgets.maxTotalJsKb) {
    checks.push(`total JS too large: ${formatKb(totalJsBytes)} > ${budgets.maxTotalJsKb.toFixed(2)} KB`);
  }

  if (toKb(totalCssBytes) > budgets.maxTotalCssKb) {
    checks.push(`total CSS too large: ${formatKb(totalCssBytes)} > ${budgets.maxTotalCssKb.toFixed(2)} KB`);
  }

  console.log("[budget] measured webview bundles:");
  console.log(
    `[budget] index-js=${indexJs ? formatKb(indexJs.bytes) : "missing"} index-css=${indexCss ? formatKb(indexCss.bytes) : "missing"}`,
  );
  console.log(
    `[budget] largest-vendor-js=${formatKb(largestVendorJs.bytes)} (${largestVendorJs.name}) total-js=${formatKb(totalJsBytes)} total-css=${formatKb(totalCssBytes)}`,
  );

  if (checks.length > 0) {
    for (const violation of checks) {
      console.error(`[budget] FAIL: ${violation}`);
    }
    process.exit(1);
  }

  console.log("[budget] PASS: all bundle budgets respected.");
}

main().catch((error) => {
  const message = error instanceof Error ? error.stack ?? error.message : String(error);
  console.error(`[budget] unexpected error: ${message}`);
  process.exit(1);
});
