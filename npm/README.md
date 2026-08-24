# ModernLink npm distribution workspace

`modernlink/` is the thin wrapper published as `@inovacc/modernlink`. Release automation must
generate each platform package with the exact same version and a `bin/modernlink` (or `.exe`)
Rust artifact before clearing `private: true`.

Packages publish only to GitHub Packages at `https://npm.pkg.github.com`. The release workflow
uses its repository-scoped `GITHUB_TOKEN` with `packages: write`; it does not use an npmjs token.
