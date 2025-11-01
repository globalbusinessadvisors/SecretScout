#!/usr/bin/env node

/**
 * Post-install script for SecretScout
 *
 * Downloads the appropriate binary from GitHub releases for the current platform
 */

const https = require('https');
const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');
const { platform, arch } = process;

const GITHUB_REPO = 'globalbusinessadvisors/SecretScout';
const PACKAGE_VERSION = require('../package.json').version;

/**
 * Map Node.js platform/arch to Rust target triples
 */
function getRustTarget() {
  const targetMap = {
    'linux-x64': 'x86_64-unknown-linux-gnu',
    'darwin-x64': 'x86_64-apple-darwin',
    'darwin-arm64': 'aarch64-apple-darwin',
    'win32-x64': 'x86_64-pc-windows-msvc',
  };

  const key = `${platform}-${arch}`;
  return targetMap[key];
}

/**
 * Get binary name for platform
 */
function getBinaryName() {
  return platform === 'win32' ? 'secretscout.exe' : 'secretscout';
}

/**
 * Download file from URL
 */
function download(url, destPath) {
  return new Promise((resolve, reject) => {
    console.log(`Downloading from ${url}`);

    https.get(url, { headers: { 'User-Agent': 'secretscout-installer' } }, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Follow redirect
        return download(response.headers.location, destPath).then(resolve).catch(reject);
      }

      if (response.statusCode !== 200) {
        reject(new Error(`Download failed with status ${response.statusCode}`));
        return;
      }

      const file = fs.createWriteStream(destPath);
      response.pipe(file);

      file.on('finish', () => {
        file.close();
        resolve();
      });

      file.on('error', (err) => {
        fs.unlink(destPath, () => {});
        reject(err);
      });
    }).on('error', reject);
  });
}

/**
 * Extract tar.gz file
 */
function extractTarGz(archivePath, destDir) {
  return new Promise((resolve, reject) => {
    const tar = spawn('tar', ['xzf', archivePath, '-C', destDir], {
      stdio: 'inherit'
    });

    tar.on('close', (code) => {
      if (code === 0) {
        fs.unlinkSync(archivePath); // Clean up archive
        resolve();
      } else {
        reject(new Error(`tar extraction failed with code ${code}`));
      }
    });

    tar.on('error', reject);
  });
}

/**
 * Extract zip file (for Windows)
 */
function extractZip(archivePath, destDir) {
  return new Promise((resolve, reject) => {
    const unzip = spawn('unzip', ['-o', archivePath, '-d', destDir], {
      stdio: 'inherit'
    });

    unzip.on('close', (code) => {
      if (code === 0) {
        fs.unlinkSync(archivePath); // Clean up archive
        resolve();
      } else {
        reject(new Error(`unzip failed with code ${code}`));
      }
    });

    unzip.on('error', reject);
  });
}

/**
 * Main installation logic
 */
async function main() {
  console.log('SecretScout: Installing binary...');

  const rustTarget = getRustTarget();
  if (!rustTarget) {
    console.warn(`\n⚠ Warning: Platform ${platform}-${arch} is not officially supported`);
    console.warn('\nSecretScout currently supports:');
    console.warn('  - Linux x64');
    console.warn('  - macOS x64 (Intel)');
    console.warn('  - macOS ARM64 (Apple Silicon)');
    console.warn('  - Windows x64');
    console.warn('\nYou can still build from source:');
    console.warn('  cargo install secretscout');
    return;
  }

  const binaryName = getBinaryName();
  const binDir = path.join(__dirname, '..', 'bin');
  const binaryPath = path.join(binDir, binaryName);

  // Check if binary already exists
  if (fs.existsSync(binaryPath)) {
    console.log('✓ Binary already installed');
    return;
  }

  // Create bin directory
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  // Download from GitHub releases
  const extension = platform === 'win32' ? 'zip' : 'tar.gz';
  const archiveName = `secretscout-${rustTarget}.${extension}`;
  const downloadUrl = `https://github.com/${GITHUB_REPO}/releases/download/v${PACKAGE_VERSION}/${archiveName}`;
  const archivePath = path.join(binDir, archiveName);

  try {
    await download(downloadUrl, archivePath);
    console.log('✓ Download complete');

    // Extract archive
    if (platform === 'win32') {
      await extractZip(archivePath, binDir);
    } else {
      await extractTarGz(archivePath, binDir);
    }
    console.log('✓ Extraction complete');

    // Make binary executable on Unix
    if (platform !== 'win32') {
      fs.chmodSync(binaryPath, 0o755);
    }

    console.log('✓ Binary installed successfully');
    console.log('\nYou can now use SecretScout:');
    console.log('  secretscout detect');
    console.log('  secretscout protect --staged');
    console.log('  secretscout --help');

  } catch (error) {
    console.error('\n✗ Installation failed:', error.message);
    console.error('\nTry installing via cargo instead:');
    console.error('  cargo install secretscout');
    console.error('\nOr download manually from:');
    console.error(`  https://github.com/${GITHUB_REPO}/releases/v${PACKAGE_VERSION}`);

    // Don't fail npm install
    process.exit(0);
  }
}

// Run main function
if (require.main === module) {
  main().catch((err) => {
    console.error('Error:', err);
    process.exit(0); // Don't fail npm install
  });
}
