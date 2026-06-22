#!/usr/bin/env node
//! Wrapper for @ffgenius/relay.
//!
//! Locates the platform-specific binary that npm installed via
//! `optionalDependencies` and execs into it, forwarding argv/stdio/exit code
//! transparently.

'use strict';

const { spawnSync } = require('child_process');

const platform = process.platform; // 'win32' | 'linux' | 'darwin' | ...
const arch = process.arch;         // 'x64' | 'arm64' | ...
const pkg = `@ffgenius/relay-${platform}-${arch}`;
const ext = platform === 'win32' ? '.exe' : '';

let exe;
try {
  // The platform package ships `bin/relay(.exe)`. require.resolve gives us
  // the absolute path npm placed it at, regardless of hoisting layout.
  exe = require.resolve(`${pkg}/bin/relay${ext}`);
} catch (err) {
  process.stderr.write(
    `relay: no binary found for ${platform}-${arch}.\n` +
    `       expected optional dependency ${pkg} to be installed.\n` +
    `       if your platform is supposed to be supported, try:\n` +
    `         npm install ${pkg}\n` +
    `       otherwise this platform is not (yet) released.\n`
  );
  process.exit(1);
}

const result = spawnSync(exe, process.argv.slice(2), {
  stdio: 'inherit',
  // On Windows .exe is a real binary, no shell needed.
  windowsHide: true,
});

if (result.error) {
  process.stderr.write(`relay: failed to spawn ${exe}: ${result.error.message}\n`);
  process.exit(1);
}

// `status` is null on signal termination; mirror that as exit 1.
process.exit(result.status === null ? 1 : result.status);
