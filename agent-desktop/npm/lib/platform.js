const { existsSync } = require('fs');
const { join } = require('path');

const PLATFORMS = {
  'win32-arm64': {
    target: 'aarch64-pc-windows-msvc',
    binaryName: 'agent-desktop-win32-arm64.exe',
    entries: ['agent-desktop.exe'],
    released: true,
  },
  'win32-x64': {
    target: 'x86_64-pc-windows-msvc',
    binaryName: 'agent-desktop-win32-x64.exe',
    entries: ['agent-desktop.exe'],
    released: true,
  },
};

function resolve(osPlatform, osArch) {
  return PLATFORMS[`${osPlatform}-${osArch}`];
}

// The first entry is always the CLI executable. Every other entry is a helper
// that is installed beside it under its own name.
function helperNames(entry) {
  return entry.entries.slice(1);
}

function releasedKeys() {
  return Object.entries(PLATFORMS)
    .filter(([, entry]) => entry.released)
    .map(([key]) => key);
}

function tarballName(version, target) {
  return `agent-desktop-v${version}-${target}.tar.gz`;
}

// Windows has bsdtar at System32\tar.exe; GNU tar via PATH misinterprets drive-letter paths.
// Resolve in-box by absolute path, fall back to PATH if missing.
function tarCommand(environment) {
  const systemRoot = environment.SystemRoot || environment.windir;
  if (!systemRoot) {
    return 'tar';
  }
  const inBox = join(systemRoot, 'System32', 'tar.exe');
  return existsSync(inBox) ? inBox : 'tar';
}

module.exports = {
  PLATFORMS,
  helperNames,
  releasedKeys,
  resolve,
  tarCommand,
  tarballName,
};
