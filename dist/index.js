#!/usr/bin/env node

// Wrapper script for Claude Code Rust
// This file is generated during build

const path = require('path');
const { spawn } = require('child_process');
const fs = require('fs');

const platform = process.platform;
const arch = process.arch;

// Path to binary
const binPath = path.join(__dirname, '..', 'bin', 'claude' + (platform === 'win32' ? '.exe' : ''));

if (!fs.existsSync(binPath)) {
    console.error('Claude Code binary not found. Please rebuild the package.');
    process.exit(1);
}

// Pass all arguments to the binary
const args = process.argv.slice(2);
const child = spawn(binPath, args, {
    stdio: 'inherit',
    shell: platform === 'win32'
});

child.on('error', (err) => {
    console.error('Failed to start Claude Code:', err.message);
    process.exit(1);
});

child.on('exit', (code) => {
    process.exit(code || 0);
});

// Handle signals
process.on('SIGINT', () => child.kill('SIGINT'));
process.on('SIGTERM', () => child.kill('SIGTERM'));