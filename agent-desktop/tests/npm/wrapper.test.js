const assert = require('node:assert/strict');
const {
  closeSync,
  cpSync,
  mkdirSync,
  openSync,
  readFileSync,
  rmSync,
  unlinkSync,
  writeFileSync,
} = require('node:fs');
const { dirname, join } = require('node:path');
const { afterEach, test } = require('node:test');

const { resolve } = require('../../npm/lib/platform.js');
const {
  createTemporaryRoots,
  osStubEnv,
  postinstallScriptPath,
  removeOsStub,
  runScriptWithOsStub,
  wrapperScriptPath,
  writeOsStub,
} = require('./helpers.js');
const { tmpdir } = require('node:os');

const { temporaryDirectory, drainRoots } = createTemporaryRoots();

test('OS stubs live in a private temporary directory', () => {
  const stub = writeOsStub('win32', 'x64');
  try {
    assert.notEqual(dirname(stub), tmpdir());
    assert.match(readFileSync(stub, 'utf8'), /const platform = process\.env/);
  } finally {
    removeOsStub(stub);
  }
});

afterEach(() => {
  // Installing now also writes a digest sidecar. Leaving one behind puts an
  // unexpected file in npm/bin, which `npm pack` would include and the package
  // contract gate rejects - so the sweep has to match what installing creates.
  try { unlinkSync(join(__dirname, '..', '..', 'npm', 'bin', '.launcher-notice-' + require('../../npm/package.json').version)); } catch {}
  for (const name of ['agent-desktop-win32-x64.exe', 'agent-desktop-win32-arm64.exe']) {
    const installed = join(__dirname, '..', '..', 'npm', 'bin', name);
    try { unlinkSync(installed); } catch {}
    try { unlinkSync(`${installed}.sha256`); } catch {}
  }
  for (const root of drainRoots()) {
    rmSync(root, { recursive: true, force: true });
  }
});

function missingBinaryWrapper() {
  const root = temporaryDirectory();
  mkdirSync(join(root, 'bin'));
  cpSync(wrapperScriptPath, join(root, 'bin/agent-desktop'));
  cpSync(join(__dirname, '../../npm/lib'), join(root, 'lib'), { recursive: true });
  return join(root, 'bin/agent-desktop');
}

test('wrapper resolves the released win32-x64 mapping and names the missing-binary cause', async () => {
  const { code, stderr } = await runScriptWithOsStub(missingBinaryWrapper(), 'win32', 'x64', ['version']);
  assert.equal(code, 1);
  assert.match(stderr, /Error: Native binary not found for win32-x64/);
  assert.match(stderr, /installed package is incomplete/);
});

test('wrapper resolves the released win32-arm64 mapping and exits non-zero when the binary is absent', async () => {
  const { code, stderr } = await runScriptWithOsStub(missingBinaryWrapper(), 'win32', 'arm64', ['version']);
  assert.equal(code, 1);
  assert.match(stderr, /Error: Native binary not found for win32-arm64/);
  assert.match(stderr, /Expected: .*agent-desktop-win32-arm64\.exe/);
});

test('wrapper refuses an unmapped platform key by name', async () => {
  const { code, stderr } = await runScriptWithOsStub(wrapperScriptPath, 'sunos', 'x64', ['version']);
  assert.equal(code, 1);
  assert.match(stderr, /Error: Unsupported platform: sunos-x64/);
});

test('wrapper refuses a darwin key by name now that only win32 is released', async () => {
  const { code, stderr } = await runScriptWithOsStub(wrapperScriptPath, 'darwin', 'arm64', ['version']);
  assert.equal(code, 1);
  assert.match(stderr, /Error: Unsupported platform: darwin-arm64/);
  assert.match(stderr, /Released platform keys today: win32-arm64, win32-x64/);
});

test('AGENT_DESKTOP_BINARY_PATH installs on windows without any helper present', () => {
  const scratch = temporaryDirectory();
  const customBinary = join(scratch, 'stand-in.exe');
  writeFileSync(customBinary, 'fake-win-binary');
  const installedBinary = join(__dirname, '..', '..', 'npm', 'bin', 'agent-desktop-win32-x64.exe');
  const stub = writeOsStub('win32', 'x64');
  let stderr = '';
  const env = { ...process.env };
  delete env.AGENT_DESKTOP_SKIP_DOWNLOAD;
  env.AGENT_DESKTOP_BINARY_PATH = customBinary;
  Object.assign(env, osStubEnv('win32', 'x64'));
  const errFile = join(scratch, 'child-stderr.txt');
  const errFd = openSync(errFile, 'w');
  try {
    require('node:child_process').execFileSync(
      process.execPath,
      ['-r', stub, postinstallScriptPath],
      { env, stdio: ['ignore', 'ignore', errFd] },
    );
  } finally {
    closeSync(errFd);
    stderr = readFileSync(errFile, 'utf8');
    removeOsStub(stub);
    try { unlinkSync(errFile); } catch {}
  }
  try {
    assert.equal(readFileSync(installedBinary, 'utf8'), 'fake-win-binary');
    assert.match(stderr, /Using binary from AGENT_DESKTOP_BINARY_PATH/);
  } finally {
    try { unlinkSync(installedBinary); } catch {}
    try { unlinkSync(`${installedBinary}.sha256`); } catch {}
  }
});
