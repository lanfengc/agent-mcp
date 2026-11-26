/**
 * Locates the native agent-browser binary for this host.
 *
 * Shared by bin/agent-browser.js and scripts/postinstall.js. Kept free of
 * subprocesses because the launcher imports it on every wrapped invocation.
 */

import { statSync } from 'fs';
import { createRequire } from 'module';
import { dirname, join } from 'path';

/**
 * Describes where the binary for this host can live:
 * - packageName/file: the per-platform optional dependency and its executable
 * - binaryName: the file name used inside this package's bin/ directory
 *   (postinstall download, source builds and linked installs)
 * - candidates: bin/ file names to try, in order
 */
export function nativeTarget() {
  return {
    packageName: '@agent-browser/cli-win32-x64-msvc',
    file: 'agent-browser.exe',
    binaryName: 'agent-browser-win32-x64.exe',
    candidates: [
      'agent-browser-win32-arm64.exe',
      'agent-browser-win32-x64.exe',
    ],
  };
}

function binarySize(path) {
  try {
    const stat = statSync(path);
    return stat.isFile() ? stat.size : null;
  } catch {
    return null;
  }
}

function platformPackageBinaryPath(packageRoot, target) {
  try {
    const require = createRequire(join(packageRoot, 'package.json'));
    const manifest = require.resolve(`${target.packageName}/package.json`);
    return join(dirname(manifest), 'bin', target.file);
  } catch {
    return null;
  }
}

// Returns existing zero-byte executables, so the launcher can name them.
export function emptyNativeBinaries(packageRoot, target) {
  const paths = target.candidates.map((name) => join(packageRoot, 'bin', name));
  const packaged = platformPackageBinaryPath(packageRoot, target);
  if (packaged) paths.push(packaged);
  return paths.filter((path) => binarySize(path) === 0);
}

// Returns the binary inside the installed platform package, or null.
export function platformPackageBinary(packageRoot, target) {
  const binary = platformPackageBinaryPath(packageRoot, target);
  return binary && binarySize(binary) > 0 ? binary : null;
}

// Returns the binary to run: a file already in bin/, else the platform package.
export function findNativeBinary(packageRoot, target) {
  for (const name of target.candidates) {
    const binary = join(packageRoot, 'bin', name);
    if (binarySize(binary) > 0) return binary;
  }
  return platformPackageBinary(packageRoot, target);
}
