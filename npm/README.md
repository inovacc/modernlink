# ModernLink npm distribution workspace

`modernlink/` is the thin wrapper published as `@inovacc/modernlink`. Release automation must
generate each platform package with the exact same version and a `bin/modernlink` (or `.exe`)
Rust artifact before clearing `private: true`.
