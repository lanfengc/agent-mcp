const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const {
  chmodSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} = require('node:fs');
const { join } = require('node:path');
const { afterEach, test } = require('node:test');

const postinstall = require('../../npm/scripts/postinstall.js');
const {
  PLATFORMS,
  helperNames,
  resolve,
  tarCommand,
  tarballName,
} = require('../../npm/lib/platform.js');
const { createTemporaryRoots, runScriptWithOsStub } = require('./helpers.js');

const postinstallScriptPath = join(__dirname, '..', '..', 'npm', 'scripts', 'postinstall.js');

const { temporaryDirectory, drainRoots } = createTemporaryRoots();

afterEach(() => {
  for (const root of drainRoots()) {
    rmSync(root, { recursive: true, force: true });
  }
});

function archive(entries) {
  const root = temporaryDirectory();
  const source = join(root, 'source');
  mkdirSync(source);
  for (const [name, contents] of Object.entries(entries)) {
    const path = join(source, name);
    writeFileSync(path, contents, { mode: 0o755 });
    chmodSync(path, 0o755);
  }
  const tarball = join(root, 'release.tar.gz');
  execFileSync(tarCommand(process.env), [
    '-czf',
    tarball,
    '-C',
    source,
    ...Object.keys(entries),
  ]);
  return { root, tarball };
}

function stagingLeftovers() {
  const packageBin = join(__dirname, '..', '..', 'npm', 'bin');
  return readdirSync(packageBin).filter((name) => name.startsWith('.extract-')).sort();
}

function captureWarnings(run) {
  const warnings = [];
  const write = process.stderr.write;
  process.stderr.write = (chunk) => {
    warnings.push(String(chunk));
    return true;
  };
  try {
    run();
    return warnings.join('');
  } finally {
    process.stderr.write = write;
  }
}

test('checksum lookup requires an exact archive name', () => {
  const hash = 'a'.repeat(64);
  assert.equal(postinstall.checksumFor(`${hash}  release.tar.gz\n`, 'release.tar.gz'), hash);
  assert.throws(
    () => postinstall.checksumFor(`${hash}  old-release.tar.gz\n`, 'release.tar.gz'),
    /Checksum entry missing/,
  );
});

test('archive validation accepts the win32 single-entry set and rejects extra payloads', () => {
  const valid = archive({ 'agent-desktop.exe': 'cli' });
  postinstall.validateArchive(valid.tarball, ['agent-desktop.exe']);

  const extra = archive({
    'agent-desktop.exe': 'cli',
    'unexpected.exe': 'payload',
  });
  assert.throws(
    () => postinstall.validateArchive(extra.tarball, ['agent-desktop.exe']),
    /unexpected entries/,
  );
});

test('archive installation places the Windows executable and removes its staging directory', () => {
  const payload = archive({ 'agent-desktop.exe': 'cli-win-build' });
  const destination = temporaryDirectory();
  const binary = join(destination, 'agent-desktop-win32-x64.exe');

  const before = readdirSync(destination);
  const stagingBefore = stagingLeftovers();
  postinstall.installArchive(payload.tarball, binary, undefined, resolve('win32', 'x64'));

  assert.equal(readFileSync(binary, 'utf8'), 'cli-win-build');
  assert.deepEqual(stagingLeftovers(), stagingBefore);
  const newEntries = readdirSync(destination).filter((name) => !before.includes(name));
  assert.deepEqual(
    newEntries.map((name) => join(destination, name)).sort(),
    [binary, `${binary}.sha256`].sort(),
  );
  assert.ok(postinstall.matchesRecordedDigest(binary));
});

test('bin entry is untouched outside node_modules', () => {
  const dir = join(temporaryDirectory(), 'checkout', 'bin');
  mkdirSync(dir, { recursive: true });
  assert.equal(postinstall.linkBinEntry(dir, 'agent-desktop-win32-x64.exe'), false);
});

test('bin entry is untouched when the native binary is missing', () => {
  const dir = join(temporaryDirectory(), 'node_modules', 'agent-desktop', 'bin');
  mkdirSync(dir, { recursive: true });
  assert.equal(postinstall.linkBinEntry(dir, 'agent-desktop-win32-x64.exe'), false);
});

test('tar resolves to the in-box Windows archiver', () => {
  const fakeSystemRoot = temporaryDirectory();
  const system32 = join(fakeSystemRoot, 'System32');
  mkdirSync(system32);
  const inBox = join(system32, 'tar.exe');
  writeFileSync(inBox, 'stand-in');

  assert.equal(tarCommand({ SystemRoot: fakeSystemRoot }), inBox);
  assert.equal(tarCommand({ windir: fakeSystemRoot }), inBox);
  assert.equal(tarCommand({}), 'tar');
  assert.equal(tarCommand({ SystemRoot: temporaryDirectory() }), 'tar');
});

test('the platform table contains only released win32 rows', () => {
  assert.deepEqual(Object.keys(PLATFORMS).sort(), ['win32-arm64', 'win32-x64']);
  assert.deepEqual(resolve('win32', 'x64'), {
    target: 'x86_64-pc-windows-msvc',
    binaryName: 'agent-desktop-win32-x64.exe',
    entries: ['agent-desktop.exe'],
    released: true,
  });
  assert.deepEqual(resolve('win32', 'arm64'), {
    target: 'aarch64-pc-windows-msvc',
    binaryName: 'agent-desktop-win32-arm64.exe',
    entries: ['agent-desktop.exe'],
    released: true,
  });
  assert.equal(resolve('darwin', 'arm64'), undefined);
  assert.equal(resolve('linux', 'x64'), undefined);
  assert.deepEqual(helperNames(PLATFORMS['win32-x64']), []);
});

test('tarball name keeps the release template', () => {
  assert.equal(
    tarballName('0.8.3', 'x86_64-pc-windows-msvc'),
    'agent-desktop-v0.8.3-x86_64-pc-windows-msvc.tar.gz',
  );
});

test('postinstall refuses unsupported platforms with exit code 0 and names the released keys', async () => {
  for (const [osPlatform, osArch] of [['freebsd', 'x64'], ['linux', 'riscv64']]) {
    const { code, stderr } = await runScriptWithOsStub(postinstallScriptPath, osPlatform, osArch);
    assert.equal(code, 0, `expected exit 0 for ${osPlatform}-${osArch}`);
    const lines = stderr.split('\n').filter((line) => line.startsWith('agent-desktop: '));
    assert.equal(
      lines[0],
      `agent-desktop: agent-desktop has no released native binary for ${osPlatform}-${osArch}.`,
    );
    assert.match(
      lines[1],
      /^agent-desktop: Released platform keys today: win32-arm64, win32-x64\.$/,
    );
    assert.equal(lines[2], 'agent-desktop: See: https://github.com/vercel-labs/agent-desktop');
  }
});

test('prompted skills cover the Windows skill and exist on disk', () => {
  const output = captureWarnings(() => postinstall.promptSkillInstall());
  const advertised = [...output.matchAll(/--skill ([a-z0-9-]+)/g)].map((match) => match[1]);
  assert.ok(advertised.includes('agent-desktop'));
  assert.ok(advertised.includes('agent-desktop-windows'));
  assert.ok(!advertised.some((name) => name.includes('macos') || name.includes('linux')));
  const skillsDir = join(__dirname, '..', '..', 'skills');
  for (const name of advertised) {
    assert.ok(existsSync(join(skillsDir, name, 'SKILL.md')), `advertised ${name} must exist`);
  }
});
