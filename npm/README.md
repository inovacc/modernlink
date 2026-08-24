# ModernLink npm distribution workspace

`modernlink/` is the thin wrapper published as `@inovacc/modernlink`. Release automation must
generate each platform package with the exact same version and a `bin/modernlink` (or `.exe`)
Rust artifact before clearing `private: true`.

Packages publish only to GitHub Packages at `https://npm.pkg.github.com`. The release workflow
uses its repository-scoped `GITHUB_TOKEN` with `packages: write`; it does not use an npmjs token.

`LATEST` is the release-version anchor. To bump every Rust crate, the npm wrapper and platform
dependency pins, managed plugin skills, and harness plugin manifests together, run:

```sh
node scripts/release_version.mjs set 0.2.0
node scripts/release_version.mjs check
```

The distribution workflow runs the same check and obtains its publish version only from `LATEST`.
