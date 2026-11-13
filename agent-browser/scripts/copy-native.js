#!/usr/bin/env node

/**
 * Copies the compiled Rust binary to bin/ with the Windows target name.
 */

import { copyFileSync, existsSync, mkdirSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';
import { arch } from 'os';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = join(__dirname, '..');

const sourcePath = join(projectRoot, 'cli/target/release/agent-browser.exe');
const binDir = join(projectRoot, 'bin');

const archKey = arch() === 'arm64' ? 'arm64' : 'x64';
const targetName = `agent-browser-win32-${archKey}.exe`;
const targetPath = join(binDir, targetName);

if (!existsSync(sourcePath)) {
  console.error(`Error: Native binary not found at ${sourcePath}`);
  console.error('Run "cargo build --release --manifest-path cli/Cargo.toml" first');
  process.exit(1);
}

if (!existsSync(binDir)) {
  mkdirSync(binDir, { recursive: true });
}

copyFileSync(sourcePath, targetPath);
console.log(`✓ Copied native binary to ${targetPath}`);
