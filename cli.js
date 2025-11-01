#!/usr/bin/env node

const { spawn } = require('child_process');
const { join } = require('path');
const { platform } = process;
const fs = require('fs');

/**
 * Find the platform-specific binary
 * The binary is downloaded during postinstall into the bin/ directory
 */
function findBinary() {
  // Binary name differs on Windows
  const binaryName = platform === 'win32' ? 'secretscout.exe' : 'secretscout';

  // Look for binary in bin/ directory
  const binaryPath = join(__dirname, 'bin', binaryName);

  if (!fs.existsSync(binaryPath)) {
    console.error(`Error: SecretScout binary not found at ${binaryPath}`);
    console.error('\nThe binary should have been downloaded during installation.');
    console.error('Try reinstalling:');
    console.error('  npm install --force secretscout');
    console.error('\nOr install from source:');
    console.error('  cargo install secretscout');
    console.error('\nOr download manually from:');
    console.error('  https://github.com/globalbusinessadvisors/SecretScout/releases/latest');
    process.exit(1);
  }

  return binaryPath;
}

/**
 * Main execution
 * Spawns the platform-specific binary with all CLI arguments passed through
 */
function main() {
  const binaryPath = findBinary();

  // Pass through all arguments (skip 'node' and script name)
  const args = process.argv.slice(2);

  // Spawn the binary as a child process
  const child = spawn(binaryPath, args, {
    stdio: 'inherit', // Inherit stdin, stdout, stderr
    windowsHide: true, // Hide console window on Windows
  });

  // Handle child process exit
  child.on('exit', (code, signal) => {
    if (signal) {
      console.error(`SecretScout was killed with signal: ${signal}`);
      process.exit(1);
    }
    process.exit(code || 0);
  });

  // Handle errors spawning the process
  child.on('error', (err) => {
    console.error('Failed to start SecretScout:', err.message);
    process.exit(1);
  });

  // Handle SIGINT (Ctrl+C) gracefully
  process.on('SIGINT', () => {
    child.kill('SIGINT');
  });

  // Handle SIGTERM gracefully
  process.on('SIGTERM', () => {
    child.kill('SIGTERM');
  });
}

// Run main function
main();
