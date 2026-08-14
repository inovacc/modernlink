# Runtime evidence — 2026-08-14

First recorded execution of any runtime path in this project. Closes **VER-05**
(native-load smoke) and **VER-04** (live HTTPS through the packaged facade).

This is a point-in-time record of what was run and what it printed. It is a machine
result, not a verdict: it establishes that the library loads and that the HTTPS path
returns a real response on this platform. It does **not** establish that the product
is correct, and only the maintainer decides whether it is good.

## Environment

| | |
|---|---|
| Commit | `58eb16a` (working tree: crate `//!` docs + `NativeLoadSmokeTest` pending) |
| Platform | Windows 11 / amd64 |
| JVM | Temurin 21.0.11 |
| Native | `C:\temp\jni\target\debug\modernlink.dll`, 24.99 MB, `cargo build -p jni@0.1.0` exit 0 in 38.52s |
| Classpath | staged dir with the DLL at `native/windows-x86_64/modernlink.dll`, exactly where `NativeLoader` looks |

**Caveat on the JVM.** This ran on Java 21, not Java 6. JNI resource selection,
extraction and `System.load` are not Java-6-specific, so this is evidence that the
native boundary works — it is **not** evidence about the Java 6 runtime itself. The
Java 6 compile gate is `docker/java6/Dockerfile`, which is a separate check. A Java 6
JRE run remains unproven.

## VER-05 — native load smoke

```
native-smoke-platform=Windows 11/amd64
native-smoke-jvm=21.0.11
native-smoke-load=ok
native-smoke-extracted-count=8
native-smoke-reload=ok
native-smoke-uuidv4=efbb8357-996a-41c7-a1ee-3001165964b2
native-smoke-uuidv7=019ffe38-9c9b-7c61-aec8-2183bf046668
native-smoke-base64=ok
native-smoke-capabilities=31
native-smoke=PASS
```

What this exercised: platform/arch resource selection, SHA-256 extraction to a
content-addressed path, `System.load`, idempotent reload, and three distinct JNI
entry-point families returning real values across the boundary (`ModernUuid`,
`ModernBase64`, `LegacyHttpClient.nativeCapabilities`). Capabilities `31` = all five
declared bits (HTTPS | TLS 1.2 | TLS 1.3 | redirects | peer certificates).

## VER-04 — live HTTPS end to end

`LegacyHttpsTest` against `https://example.com`, TLS terminated and verified in Rust:

```
status=200
final-url=https://example.com
body-bytes=559
peer-certs=4
tls-protocol=TLSv1_3
tls-cipher=TLS13_AES_256_GCM_SHA384
```

Through the `HttpsURLConnection`-shaped adapter (`ModernHttpsURLConnectionTest`):

```
adapter-status=200
adapter-body-bytes=559
adapter-cipher=TLS13_AES_256_GCM_SHA384
adapter-certificates=4
```

POST paths (`LegacyHttpsPostTest`, `ModernHttpsURLConnectionPostTest`) returned `405`
from example.com, which is the expected server response to a POST there — the point is
that the request crossed the boundary and a real status came back.

## Full Java suite — all 11 classes

```
NativeLoadSmokeTest                  PASS  exit=0
ModernUtilityStandaloneTest          PASS  exit=0
LegacyHttpCapabilitiesTest           PASS  exit=0
LegacyHttpRequestTest                PASS  exit=0
LegacyHttpRequestConfigurationTest   PASS  exit=0
LegacyHttpRequestRedirectTest        PASS  exit=0
LegacyHttpResponseStructuredTest     PASS  exit=0
LegacyHttpsTest                      PASS  exit=0
LegacyHttpsPostTest                  PASS  exit=0
ModernHttpsURLConnectionTest         PASS  exit=0
ModernHttpsURLConnectionPostTest     PASS  exit=0
TOTAL: 11 classes  PASS=11  FAIL=0
```

CI currently invokes only three of these (`.github/workflows/test.yml:31-33`) — see
**VER-03**.

## What this does NOT establish

- **Nothing about messaging.** No broker was contacted; `crates/messaging` beyond
  `InMemoryTransport` remains a source-level claim (ISSUES I-010).
- **Nothing about Java 6.** See the JVM caveat above.
- **Nothing about the other two platforms.** linux-x86_64 and linux-aarch64 natives are
  cross-compiled but have never been loaded. Deferred by decision — Windows first.
- **Nothing about the routing policy engine**, which no Java caller can reach at all
  (BUGS B-002).
- The CI Rust gate is still red (BUGS B-001); these runs were local.

## Reproduce

```powershell
cargo build -p jni@0.1.0
pwsh .scripts/04-D_stage_all_tests.ps1
pwsh .scripts/05-D_run_all_java_tests.ps1
```
