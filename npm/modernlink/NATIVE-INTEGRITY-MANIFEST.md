# Native integrity manifest

Each published `@inovacc/modernlink-<platform>-<arch>` package must contain
`modernlink-native.json` beside its `package.json`:

```json
{
  "schema_version": "modernlink.native-integrity/v1",
  "package": "@inovacc/modernlink-win32-x64",
  "version": "0.1.0",
  "os": "win32",
  "arch": "x64",
  "binary_path": "bin/modernlink.exe",
  "binary_sha256": "sha256:<64 lowercase hex characters>"
}
```

Release automation computes the hash from the final packaged Rust executable. The wrapper checks
every field, rejects path traversal, requires a regular file, and verifies SHA-256 before exec.
The npm registry package integrity protects package acquisition; this manifest additionally binds
the launcher to the exact native executable it will run.
