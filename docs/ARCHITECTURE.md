# Architecture
<!-- rev:004 (RFC 3339) 2026-08-21T00:00:00Z -->

Diagrams reflect the current component boundaries. Execution reach is tracked separately in
[VERIFICATION.md](VERIFICATION.md); a drawn path is not runtime evidence.

## System overview

```mermaid
flowchart TB
    subgraph legacy["Legacy runtime (Java 6, vendor-locked)"]
        APP["Host application"]
    end

    subgraph jar["modernlink.jar — Java 6 facade (-source 1.6 -target 1.6)"]
        HTTPF["LegacyHttpClient / LegacyHttpRequest<br/>ModernHttpsURLConnection"]
        MSGF["ModernConnectionFactory / Connection<br/>Session / Producer / Consumer"]
        UTIL["ModernUuid · ModernBase64 · ModernJson"]
        JMX["ModernMessagingMetricsMBean"]
        LOADER["NativeLoader<br/>SHA-256 content-addressed extraction"]
    end

    subgraph native["libmodernlink — Rust native library"]
        JNIC["crates/jni<br/>28 Java_* entry points"]
        CORE["crates/core<br/>Request · Response · TlsInfo · Error"]
        HTTP["crates/http<br/>hyper HTTP/1.1"]
        TLS["crates/tls<br/>rustls, floor TLS 1.2"]
        MSG["crates/messaging<br/>uniform transport boundary"]
    end

    subgraph brokers["External systems"]
        HTTPS["HTTPS endpoints"]
        NATS["NATS / JetStream"]
        KAFKA["Kafka"]
        PULSAR["Pulsar"]
        RABBIT["RabbitMQ"]
    end

    APP --> HTTPF & MSGF & UTIL
    MSGF -.reads.-> JMX
    HTTPF & MSGF & UTIL --> LOADER
    LOADER -->|"System.load"| JNIC
    JNIC --> CORE & HTTP & TLS & MSG
    HTTP --> TLS
    TLS ==>|"TLS 1.2 / 1.3"| HTTPS
    MSG --> NATS & KAFKA & PULSAR & RABBIT
    MSG -->|"InMemoryTransport"| LEGACY["LEGACY_JMS<br/>in-process only — not a vendor broker bridge"]

    style LEGACY stroke-dasharray: 5 5
```

## HTTPS request flow

```mermaid
sequenceDiagram
    participant App as Java 6 app
    participant Facade as ModernHttpsURLConnection
    participant JNI as crates/jni
    participant Http as crates/http
    participant Tls as crates/tls
    participant Peer as HTTPS endpoint

    App->>Facade: new ModernHttpsURLConnection(url)
    App->>Facade: setRequestProperty / setConnectTimeout
    App->>Facade: getInputStream()
    Facade->>JNI: nativeExecute(request)
    JNI->>Http: execute(Request)
    Http->>Tls: client config (min TLS 1.2)

    alt handshake + response
        Tls-->>Peer: TLS handshake
        Peer-->>Http: status, headers, body
        Http-->>JNI: Response + TlsInfo
        JNI-->>Facade: handle
        Facade->>JNI: nativeStatus / nativeHeaders / nativeBody / nativeTlsProtocol
        Facade-->>App: InputStream + status + headers
    else failure
        Http-->>JNI: Error
        JNI-->>Facade: nativeLastError()
        Facade-->>App: throw LegacyHttpException
    end

    App->>Facade: close / disconnect
    Facade->>JNI: nativeRelease(handle)
```

Timeouts cover both TCP connection and the TLS handshake. Custom `HostnameVerifier` and
`SSLSocketFactory` are rejected rather than ignored — Rust terminates and verifies TLS, so
accepting them would be a misleading security contract.

## Messaging publish / receive / acknowledge

```mermaid
sequenceDiagram
    participant App as Java 6 app
    participant Fac as ModernMessagingClient
    participant JNI as crates/jni
    participant Msg as crates/messaging
    participant Broker as Provider transport

    App->>Fac: createConnection(provider, mode)
    Fac->>JNI: nativeOpen(provider, endpoint)
    JNI->>Msg: transport for Provider
    Msg-->>JNI: client handle

    App->>Fac: producer.send(ModernTextMessage)
    Fac->>JNI: nativePublish(envelope + trace context)
    JNI->>Msg: publish(envelope)

    alt routing policy accepts
        Msg->>Broker: provider-encoded message
        Broker-->>Msg: broker ack
        Msg-->>JNI: DeliveryReceipt(state)
        JNI-->>Fac: receipt
        Fac-->>App: ModernDeliveryReceipt
    else provider mismatch / policy reject
        Msg-->>JNI: DomainError
        JNI-->>Fac: nativeLastError()
        Fac-->>App: throw — fail closed, never silent
    end

    loop consumer
        App->>Fac: consumer.receive()
        Fac->>JNI: nativeReceive()
        JNI->>Msg: receive()
        Msg->>Broker: poll / pull
        Broker-->>Msg: message
        Msg-->>Fac: ModernReceivedMessage (trace context preserved)
        App->>Fac: acknowledge()
        Fac->>JNI: nativeAcknowledge()
        JNI->>Msg: ack
        Msg->>Broker: server-side ack
    end

    App->>Fac: close()
    Fac->>JNI: nativeClose()
```

## Native library lifecycle

```mermaid
sequenceDiagram
    participant JVM as Java 6 JVM
    participant Loader as NativeLoader
    participant FS as Filesystem
    participant Lib as libmodernlink

    JVM->>Loader: static initializer
    Loader->>Loader: detect OS + architecture
    Loader->>Loader: select native/<os>-<arch>/ resource

    alt resource present
        Loader->>Loader: SHA-256 the embedded bytes
        Loader->>FS: write unique temp file
        Loader->>FS: rename to content-addressed path
        Note over FS: a later JVM reuses this file
        Loader->>JVM: System.load(path)
        JVM->>Lib: JNI_OnLoad
    else extraction or load fails
        Loader->>FS: delete temp file
        Loader-->>JVM: UnsatisfiedLinkError with context
    end
```

## Build and packaging

```mermaid
flowchart LR
    subgraph stage1["Stage 1 — rust:1.96-bookworm"]
        SRC["Cargo workspace"] --> ZIG["cargo-zigbuild<br/>cmake · protobuf-compiler"]
        ZIG --> L1["linux-x86_64<br/>libmodernlink.so"]
        ZIG --> L2["linux-aarch64<br/>libmodernlink.so"]
        ZIG --> L3["windows-x86_64<br/>modernlink.dll"]
    end

    subgraph stage2["Stage 2 — java:6b38-jdk"]
        JSRC["java/src"] --> JAVAC["javac -source 1.6 -target 1.6"]
        L1 & L2 & L3 --> RES["build/classes/native/..."]
        JAVAC --> CLS["build/classes"]
        RES --> JAR
        CLS --> JAR["jar cf modernlink.jar"]
    end

    style stage2 fill:none
```

`docker/java6/Dockerfile` is the only supported way to compile and package the Java facade —
there is no Maven or Gradle build. The `java:6b38-jdk` base image is deprecated and may not be
pullable from every registry; see [ISSUES.md](ISSUES.md).

## Source layout

```text
crates/core           package `modernlink-core` - shared request/response, TLS metadata, errors
crates/http           HTTPS execution (hyper)
crates/tls            TLS policy boundary (rustls, webpki-roots)
crates/messaging      InMemory · NATS · JetStream · Kafka · Pulsar · RabbitMQ transports,
                      each behind a cargo feature; default = none (SC-07)
crates/jni            package `jni-bridge` - 28 Java_* entry points; builds libmodernlink
hacks/messaging-demo  executable contract fixtures (Rust, 7 binaries)
hacks/java6-messaging Java 6 JMS/JMX-shaped fixture (4 classes)
java/src/main/java    com.modernlink facade (35 classes)
java/src/test/java    standalone main-style tests (15 classes)
docker/java6          the packaging build

The two package names differ from their folders on purpose: bare `core` shadowed Rust's
built-in crate and bare `jni` shadowed the external `jni` dependency. See ISSUES I-001/I-002.
```
