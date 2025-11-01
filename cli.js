#!/usr/bin/env node

const { spawn } = require('child_process');
const { join } = require('path');
const { platform, arch } = process;

/**
 * Map Node.js platform/arch to npm package names
 * These correspond to the platform-specific optional dependency packages
 */
function getPlatformPackageName() {
  const platformMap = {
    'linux-x64': '@secretscout/linux-x64',
    'darwin-x64': '@secretscout/darwin-x64',
    'darwin-arm64': '@secretscout/darwin-arm64',
    'win32-x64': '@secretscout/win32-x64',
  };

  const key = `${platform}-${arch}`;
  return platformMap[key];
}

/**
 * Find the platform-specific binary
 * Tries to locate the binary from the optional dependency package
 */
function findBinary() {
  const packageName = getPlatformPackageName();

  if (!packageName) {
    console.error(`Error: Unsupported platform ${platform}-${arch}`);
    console.error('SecretScout currently supports:');
    console.error('  - Linux x64');
    console.error('  - macOS x64 (Intel)');
    console.error('  - macOS ARM64 (Apple Silicon)');
    console.error('  - Windows x64');
    console.error('\nPlease build from source: https://github.com/globalbusinessadvisors/SecretScout');
    process.exit(1);
  }

  // Binary name differs on Windows
  const binaryName = platform === 'win32' ? 'secretscout.exe' : 'secretscout';

  try {
    // Try to resolve the platform-specific package
    const binaryPath = require.resolve(`${packageName}/${binaryName}`);
    return binaryPath;
  } catch (err) {
    console.error(`Error: Failed to find SecretScout binary for ${platform}-${arch}`);
    console.error(`\nExpected package: ${packageName}`);
    console.error('\nTry running:');
    console.error('  npm install --force');
    console.error('\nOr install from source:');
    console.error('  cargo install secretscout');
    console.error('\nOriginal error:', err.message);
    process.exit(1);
  }
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
