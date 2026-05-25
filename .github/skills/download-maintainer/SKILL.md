---
name: download-maintainer
description: Maintain durl/download-lib/libdurl with safe release workflow
---

# Download Maintainer Skill

Use this skill when working in this repository to make consistent changes across `durl`, `download-lib`, and `libdurl`.

## When To Use

- Adding or changing CLI parameters in `src/main.rs`
- Updating download behavior in `download-lib/src/lib.rs`
- Keeping C ABI wrapper (`libdurl/src/lib.rs`) compatible
- Preparing crates for publish
- Updating docs and usage examples

## Core Rules

1. If `DownloadFile::start_download` signature changes, update all call sites:
   - `src/main.rs`
   - `libdurl/src/lib.rs`
2. Keep crate versions aligned:
   - `download-lib/Cargo.toml` version
   - root `Cargo.toml` dependency on `download-lib`
3. For publishable manifests, local path deps must also include `version`.
4. Preserve backward compatibility for FFI where possible.
5. Update README examples when CLI flags change.

## Safe Change Checklist

- [ ] Search all `start_download(` usages and update signatures
- [ ] Run `cargo build` in root crate
- [ ] Run `cargo build` in `libdurl/`
- [ ] Verify `README.md` examples still match runtime flags
- [ ] Verify publish manifests contain version requirements

## Common Commands

```bash
cargo build
cargo check
```

```bash
cd libdurl
cargo build
```

```bash
cargo publish --dry-run
```

```bash
cd download-lib
cargo publish --dry-run
```

## Troubleshooting

### Error: dependency does not specify a version

Cause: using only path dependency in a publishable crate.

Fix:

```toml
download-lib = { path = "download-lib", version = "0.2.5" }
```

### Error: function takes N arguments but M supplied

Cause: API signature changed in local crate but dependent crate still points to crates.io version.

Fix options:

1. During local development, temporarily use path dependency.
2. Or publish `download-lib` first, then update dependent crate version.

