# Platform-Specific npm Packages

This directory contains templates for platform-specific npm packages.

## How It Works

When cargo-dist runs during the release process:

1. It builds the SecretScout binary for each target platform
2. It copies the binary into the appropriate directory here:
   - `linux-x64/secretscout` - Linux x86_64 binary
   - `darwin-x64/secretscout` - macOS x86_64 binary
   - `darwin-arm64/secretscout` - macOS ARM64 binary
   - `win32-x64/secretscout.exe` - Windows x86_64 binary

3. It publishes each directory as a separate npm package:
   - `@secretscout/linux-x64`
   - `@secretscout/darwin-x64`
   - `@secretscout/darwin-arm64`
   - `@secretscout/win32-x64`

4. The main `secretscout` package lists these as optional dependencies

## Directory Structure

Each platform directory contains:
- `package.json` - npm package metadata with OS/CPU constraints
- `index.js` - Simple module export (for npm compatibility)
- Binary will be added by cargo-dist during build

## Manual Publishing

If you need to manually publish a platform package:

```bash
# Build the binary for the target platform
cargo build --release --target <rust-target>

# Copy to the appropriate directory
cp target/<rust-target>/release/secretscout npm/<platform>/

# Publish
cd npm/<platform>
npm publish --access public
```

## Notes

- These packages are intentionally kept minimal
- The binaries are NOT checked into git (they're in .gitignore)
- cargo-dist handles the entire build and publish process automatically
- Manual publishing is only needed for testing or emergency releases
