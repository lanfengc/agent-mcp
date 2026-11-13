#!/usr/bin/env node
const { execSync } = require('child_process');
const { existsSync, readFileSync } = require('fs');
const { join } = require('path');
const { tmpdir } = require('os');

const root = join(__dirname, '..');
const npmDir = join(root, 'npm');
const pkg = require(join(npmDir, 'package.json'));
const { PLATFORMS } = require(join(npmDir, 'lib', 'platform.js'));
const releaseWorkflow = readFileSync(join(root, '.github/workflows/release.yml'), 'utf8')
  .replace(/\r\n/g, '\n');

const WINDOWS_TARGETS = ['aarch64-pc-windows-msvc', 'x86_64-pc-windows-msvc'];
const expectedFiles = [
  'bin/agent-desktop',
  'lib/platform.js',
  'lib/windows-bin.js',
  'package.json',
  'scripts/postinstall.js',
];

const failures = [];
const check = (condition, message) => {
  if (!condition) failures.push(message);
};
const sameSet = (actual, expected) =>
  JSON.stringify([...actual].sort()) === JSON.stringify([...expected].sort());

for (const file of expectedFiles) {
  check(existsSync(join(npmDir, file)), `npm package is missing ${file}`);
}

check(
  sameSet(pkg.os ?? [], ['win32']),
  `npm package os must be exactly ["win32"], got ${JSON.stringify(pkg.os)}`,
);
check(pkg.bin?.['agent-desktop'] === 'bin/agent-desktop', 'npm bin path must be bin/agent-desktop');
check(
  pkg.repository?.url === 'git+https://github.com/vercel-labs/agent-desktop.git',
  'npm repository URL must be normalized for npm publish',
);

const platformKeys = Object.keys(PLATFORMS);
const nonWindowsKeys = platformKeys.filter((key) => !key.startsWith('win32-'));
check(
  nonWindowsKeys.length === 0,
  `npm platform table must contain only win32 rows, found ${nonWindowsKeys.join(', ')}`,
);
check(
  sameSet(Object.values(PLATFORMS).map((entry) => entry.target), WINDOWS_TARGETS),
  `npm platform table must target exactly ${WINDOWS_TARGETS.join(' and ')}`,
);

// Returns the YAML block for one job so a matrix or permission is asserted
// about the job that needs it rather than about the file that contains it.
function jobBlock(workflow, jobName) {
  const lines = workflow.split('\n');
  const start = lines.findIndex((line) => line === `  ${jobName}:`);
  if (start === -1) {
    return null;
  }
  const rest = lines.slice(start + 1);
  const end = rest.findIndex((line) => /^ {2}\S/.test(line));
  return (end === -1 ? rest : rest.slice(0, end)).join('\n');
}

const buildJob = jobBlock(releaseWorkflow, 'build');
check(buildJob !== null, 'release workflow must define a build job');
if (buildJob !== null) {
  const buildTargets = [...buildJob.matchAll(/^\s*-\s+target:\s*(\S+)/gm)].map(
    (match) => match[1],
  );
  check(
    sameSet(buildTargets, WINDOWS_TARGETS),
    `release build matrix must contain exactly ${WINDOWS_TARGETS.join(' and ')}, got ${buildTargets.join(', ')}`,
  );
}
check(jobBlock(releaseWorkflow, 'publish-npm') !== null, 'release workflow must define a publish-npm job');
check(
  jobBlock(releaseWorkflow, 'publish-github') !== null,
  'release workflow must define a publish-github job',
);

if (failures.length > 0) {
  throw new Error(`Windows-only npm package and release workflow checks failed:\n${failures.join('\n')}`);
}

const output = execSync('npm pack --dry-run --json', {
  cwd: npmDir,
  encoding: 'utf8',
  env: {
    ...process.env,
    npm_config_cache:
      process.env.npm_config_cache || join(tmpdir(), 'agent-desktop-npm-cache'),
  },
});

// `npm pack --json` reported an array of packed packages up to npm 10 and
// reports an object keyed by package name from npm 11 on. Accept both.
const packed = JSON.parse(output);
const pack = Array.isArray(packed) ? packed[0] : Object.values(packed)[0];
if (!pack || !Array.isArray(pack.files)) {
  throw new Error(
    'npm pack --json returned an unrecognised shape; this gate cannot verify the packed file list',
  );
}

const bundled = process.argv.includes('--bundled');
const expected = [
  ...expectedFiles,
  ...(bundled ? require('./npm-bundled-files')() : []),
].sort();
const actual = pack.files.map((file) => file.path).sort();

if (JSON.stringify(actual) !== JSON.stringify(expected)) {
  throw new Error(`Unexpected npm package contents: ${actual.join(', ')}`);
}
if (pack.bundled && pack.bundled.length > 0) {
  throw new Error(`npm package unexpectedly bundles dependencies: ${pack.bundled.join(', ')}`);
}
if (pack.unpackedSize > (bundled ? 100_000_000 : 25_000)) {
  throw new Error(`npm package is unexpectedly large: ${pack.unpackedSize} bytes`);
}

console.log(
  `OK: Windows-only npm package contains ${actual.length} files, ${pack.unpackedSize} bytes unpacked`,
);
