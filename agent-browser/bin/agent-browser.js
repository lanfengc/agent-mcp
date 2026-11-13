#!/usr/bin/env node

/**
 * CLI launcher for agent-browser on Windows
 *
 * Runs the native binary from the platform package (the optional dependency
 * @agent-browser/cli-win32-x64-msvc) or from this package's bin/. On first
 * use it also points the package manager's bin entry at the native binary,
 * so later invocations skip Node.js entirely, even when install scripts were
 * blocked.
 */

import { spawn } from 'child_process';
import { dirname } from 'path';
import { fileURLToPath } from 'url';
import { findNativeBinary, nativeTarget } from '../scripts/native.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));

async function main() {
  const target = nativeTarget();

  const binaryPath = findNativeBinary(packageRoot, target);

  if (!binaryPath) {
    console.error(`Error: No binary found for Windows.`);
    console.error(`Expected the ${target.packageName} package to be installed.`);
    console.error('');
    console.error('Reinstall agent-browser with optional dependencies enabled,');
    console.error('or run "npm run build:native" to build for your platform.');
    process.exit(1);
  }

  let runPath = binaryPath;
  try {
    const { linkNative, materializeBinary } = await import('../scripts/link-native.js');
    try {
      runPath = materializeBinary(packageRoot, binaryPath, target);
    } catch {
      runPath = binaryPath;
    }
    linkNative({ packageRoot, binary: runPath, target, entries: [process.argv[1]] });
  } catch {
    // Linking is an optimization; this launcher keeps working without it.
  }

  // Spawn the native binary with inherited stdio
  const child = spawn(runPath, process.argv.slice(2), {
    stdio: 'inherit',
    windowsHide: false,
  });

  child.on('error', (err) => {
    console.error(`Error executing binary: ${err.message}`);
    process.exit(1);
  });

  child.on('close', (code) => {
    process.exit(code ?? 0);
  });
}

main();
