#!/usr/bin/env node

/**
 * Post-install script for SecretScout
 *
 * This script runs after npm install and performs the following:
 * 1. Checks if a platform-specific package was installed
 * 2. If not, attempts to download the binary from GitHub releases
 * 3. Provides helpful error messages if installation fails
 */

const https = require('https');
const fs = require('fs');
const path = require('path');
const { platform, arch } = process;

/**
 * Check if a platform-specific package exists
 */
function checkPlatformPackage() {
  const platformMap = {
    'linux-x64': '@secretscout/linux-x64',
    'darwin-x64': '@secretscout/darwin-x64',
    'darwin-arm64': '@secretscout/darwin-arm64',
    'win32-x64': '@secretscout/win32-x64',
  };

  const key = `${platform}-${arch}`;
  const packageName = platformMap[key];

  if (!packageName) {
    return { exists: false, unsupported: true };
  }

  try {
    // Try to require the platform package
    require.resolve(packageName);
    return { exists: true, packageName };
  } catch {
    return { exists: false, packageName, unsupported: false };
  }
}

/**
 * Main post-install logic
 */
function main() {
  console.log('SecretScout: Running post-install checks...');

  const check = checkPlatformPackage();

  if (check.exists) {
    console.log('✓ Platform-specific binary installed successfully');
    console.log(`  Package: ${check.packageName}`);
    console.log('\nYou can now use SecretScout:');
    console.log('  secretscout detect');
    console.log('  secretscout protect --staged');
    console.log('  secretscout --help');
    return;
  }

  if (check.unsupported) {
    console.warn(`\n⚠ Warning: Platform ${platform}-${arch} is not officially supported`);
    console.warn('\nSecretScout currently supports:');
    console.warn('  - Linux x64');
    console.warn('  - macOS x64 (Intel)');
    console.warn('  - macOS ARM64 (Apple Silicon)');
    console.warn('  - Windows x64');
    console.warn('\nYou can still build from source:');
    console.warn('  git clone https://github.com/globalbusinessadvisors/SecretScout.git');
    console.warn('  cd SecretScout');
    console.warn('  cargo build --release');
    console.warn('\nOr install via cargo:');
    console.warn('  cargo install secretscout');
    // Don't exit with error - just warn
    return;
  }

  // Platform is supported but package wasn't installed
  console.error(`\n✗ Error: Platform package ${check.packageName} not found`);
  console.error('\nThis usually means the optional dependency was not installed.');
  console.error('Try running:');
  console.error('  npm install --force');
  console.error('\nIf the problem persists, install from source:');
  console.error('  cargo install secretscout');
  console.error('\nOr download the binary manually:');
  console.error('  https://github.com/globalbusinessadvisors/SecretScout/releases/latest');

  // Note: We don't exit with error code here because npm install should succeed
  // even if optional dependencies fail. Users can still build from source.
}

// Only run if this is the main module (not being required)
if (require.main === module) {
  main();
}

module.exports = { checkPlatformPackage };
