import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmod, copyFile, link, mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const repositoryRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const launcherPath = join(repositoryRoot, 'bin', 'agent-browser.js');
const sharedModules = ['native.js', 'link-native.js'];

// Creates a package directory holding the launcher and the modules it imports.
async function createPackage(t, packageRoot) {
  if (!packageRoot) {
    packageRoot = await mkdtemp(join(tmpdir(), 'agent-browser-launcher-'));
    t.after(() => rm(packageRoot, { recursive: true, force: true }));
  }
  await mkdir(join(packageRoot, 'bin'), { recursive: true });
  await mkdir(join(packageRoot, 'scripts'), { recursive: true });
  await writeFile(join(packageRoot, 'package.json'), '{"type":"module"}\n');
  await copyFile(launcherPath, join(packageRoot, 'bin', 'agent-browser.js'));
  await chmod(join(packageRoot, 'bin', 'agent-browser.js'), 0o755);
  for (const name of sharedModules) {
    await copyFile(join(repositoryRoot, 'scripts', name), join(packageRoot, 'scripts', name));
  }
  return packageRoot;
}

async function writeFakeBinary(path, marker) {
  await mkdir(dirname(path), { recursive: true });
  if (process.platform === 'win32') {
    await link(process.execPath, path).catch(() => copyFile(process.execPath, path));
    await writeFile(`${path}.marker`, marker);
    return;
  }
  await writeFile(
    path,
    `#!/bin/sh
printf '%s\\n' ${JSON.stringify(marker)} "$@"
`,
  );
  await chmod(path, 0o755);
}

// Installs a fake platform package, as npm does for optionalDependencies.
async function writePlatformPackage(packageRoot, packageName, file, marker) {
  const directory = join(packageRoot, 'node_modules', ...packageName.split('/'));
  await mkdir(directory, { recursive: true });
  await writeFile(join(directory, 'package.json'), JSON.stringify({ name: packageName }));
  await writeFakeBinary(join(directory, 'bin', file), marker);
  return join(directory, 'bin', file);
}

async function runLauncher(t, { platform, arch, binaries = [], platformPackage, args }) {
  const packageRoot = await createPackage(t);
  const bootstrapPath = join(packageRoot, 'bootstrap.mjs');
  const fakeStubPath = join(packageRoot, 'fake-binary.cjs');

  await writeFile(
    fakeStubPath,
    `const fs = require('node:fs');
const path = require('node:path');
function markerPath() {
  const direct = process.argv0 + '.marker';
  if (fs.existsSync(direct)) return direct;
  const pending = [process.cwd()];
  while (pending.length > 0) {
    const directory = pending.pop();
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const full = path.join(directory, entry.name);
      if (entry.isDirectory()) pending.push(full);
      else if (entry.name.endsWith('.marker')) return full;
    }
  }
  return direct;
}
console.log(fs.readFileSync(markerPath(), 'utf8').trimEnd());
for (const arg of process.argv.slice(2)) console.log(arg);
`,
  );

  await writeFile(
    bootstrapPath,
    `import { createRequire, syncBuiltinESMExports } from 'node:module';

const require = createRequire(import.meta.url);
const os = require('node:os');
os.platform = () => ${JSON.stringify(platform)};
os.arch = () => ${JSON.stringify(arch)};
syncBuiltinESMExports();

await import('./bin/agent-browser.js');
`,
  );
  for (const { name, marker } of binaries) {
    await writeFakeBinary(join(packageRoot, 'bin', name), marker);
  }
  if (platformPackage) {
    await writePlatformPackage(packageRoot, platformPackage.name, platformPackage.file, platformPackage.marker);
  }

  const spawnArgs = process.platform === 'win32' ? [fakeStubPath, ...args] : args;
  return spawnSync(process.execPath, [bootstrapPath, ...spawnArgs], {
    cwd: packageRoot,
    encoding: 'utf8',
  });
}

test('Windows ARM64 launcher uses the published x64 executable', async (t) => {
  const args = ['open', 'https://example.com'];
  const result = await runLauncher(t, {
    platform: 'win32',
    arch: 'arm64',
    binaries: [
      { name: 'agent-browser-win32-x64.exe', marker: 'x64-binary-ran' },
    ],
    args,
  });

  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(result.stdout.trim().split('\n'), ['x64-binary-ran', ...args]);
  assert.doesNotMatch(result.stderr, /No binary found/);
});

test('Windows ARM64 launcher prefers an existing native executable', async (t) => {
  const args = ['--version'];
  const result = await runLauncher(t, {
    platform: 'win32',
    arch: 'arm64',
    binaries: [
      { name: 'agent-browser-win32-arm64.exe', marker: 'arm64-binary-ran' },
      { name: 'agent-browser-win32-x64.exe', marker: 'x64-binary-ran' },
    ],
    args,
  });

  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(result.stdout.trim().split('\n'), ['arm64-binary-ran', ...args]);
});

test('Windows ARM64 launcher runs when only a source-built native executable exists', async (t) => {
  const args = ['snapshot'];
  const result = await runLauncher(t, {
    platform: 'win32',
    arch: 'arm64',
    binaries: [
      { name: 'agent-browser-win32-arm64.exe', marker: 'arm64-binary-ran' },
    ],
    args,
  });

  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(result.stdout.trim().split('\n'), ['arm64-binary-ran', ...args]);
  assert.doesNotMatch(result.stderr, /No binary found/);
});

test('Windows launcher runs the binary from the platform package', async (t) => {
  const args = ['open', 'https://example.com'];
  const result = await runLauncher(t, {
    platform: 'win32',
    arch: 'arm64',
    platformPackage: { name: '@agent-browser/cli-win32-x64-msvc', file: 'agent-browser.exe', marker: 'win32-package-ran' },
    args,
  });

  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(result.stdout.trim().split('\n'), ['win32-package-ran', ...args]);
});

test('launcher identifies an empty platform executable', async (t) => {
  const packageRoot = await createPackage(t);
  const packageName = '@agent-browser/cli-win32-x64-msvc';
  const platformDir = join(packageRoot, 'node_modules', ...packageName.split('/'));
  const emptyBinary = join(platformDir, 'bin', 'agent-browser.exe');
  await mkdir(dirname(emptyBinary), { recursive: true });
  await writeFile(join(platformDir, 'package.json'), JSON.stringify({ name: packageName }));
  await writeFile(emptyBinary, '');

  const result = spawnSync(process.execPath, [join(packageRoot, 'bin', 'agent-browser.js'), '--version'], {
    encoding: 'utf8',
  });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /0 bytes/);
  assert.ok(result.stderr.includes(emptyBinary), result.stderr);
});

test('launcher reports a missing platform package', async (t) => {
  const result = await runLauncher(t, { platform: 'win32', arch: 'arm64', args: ['--version'] });

  assert.equal(result.status, 1);
  assert.match(result.stderr, /@agent-browser\/cli-win32-x64-msvc/);
});
