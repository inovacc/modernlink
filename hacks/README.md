# ModernLink hacks

This directory contains executable compatibility fixtures. They are not part
of the distributable JAR. The NATS fixtures exercise a real external NATS
broker; Kafka, Pulsar, RabbitMQ, and JMS broker interoperability remain open.

## Rust cross-application fixture

The Cargo package `messaging-demo` has two separate applications:

- `legacy-jms-app`: creates a JMS/JMX-shaped publishing frame;
- `modern-provider-app`: consumes the same frame as a modern provider.

From the repository root:

```powershell
cargo run --manifest-path hacks/messaging-demo/Cargo.toml --bin legacy-jms-app -- transform kafka |
    cargo run --manifest-path hacks/messaging-demo/Cargo.toml --bin modern-provider-app -- kafka

cargo run --manifest-path hacks/messaging-demo/Cargo.toml --bin legacy-jms-app -- redirect nats |
    cargo run --manifest-path hacks/messaging-demo/Cargo.toml --bin modern-provider-app -- nats
```

The output reports the selected mode, provider, destination, message ID,
payload kind, acknowledgement mode, and first-class tracing fields. The
provider-neutral domain also exposes typed publish/receive/acknowledge
receipts; the fixtures validate that the publish receipt matches the routed
message and provider. The current in-memory transport is a contract fixture,
not an external broker adapter.

## NATS broker fixture

The main messaging crate includes a Cargo-backed `NatsTransport`. Run a local
NATS server and then the dedicated fixture:

```powershell
docker run --rm -d --name modernlink-nats -p 4222:4222 nats:2.10-alpine
cargo run --manifest-path hacks/messaging-demo/Cargo.toml --bin nats-app
docker rm -f modernlink-nats
```

The NATS core transport reports publish, receive, and local acknowledgement
receipts. Core NATS does not provide durable broker acknowledgements; the
separate JetStream transport below provides server-side acknowledgement state.

The JetStream adapter is exercised separately with a durable stream and pull
consumer:

```powershell
docker run --rm -d --name modernlink-nats-js -p 4222:4222 nats:2.10-alpine -js
cargo run --manifest-path hacks/messaging-demo/Cargo.toml --bin jetstream-app
docker rm -f modernlink-nats-js
```

Its receipt is returned only after the server-side JetStream acknowledgement.

The Java facade can exercise the same native boundary on a host whose bundled
native resource matches the host platform:

```powershell
javac -source 1.6 -target 1.6 ...
java -cp hacks/java6-messaging/classes com.modernlink.messaging.NativeNatsApp
```

`NativeNatsApp` uses `ModernMessagingClient` and preserves the message and
trace identities across publish, receive, and acknowledgement. The current
fixture has a Windows x86_64 native probe; Java 6 Docker compilation and the
Linux packaged-JAR probe remain separate checks.

`JmsFacadeNatsApp` exercises the JMS-shaped layer (`ConnectionFactory`,
`Connection`, `Session`, `MessageProducer`, and `MessageConsumer`) and registers
the shared `ModernMessagingMetricsMBean`:

```powershell
java -cp hacks/java6-messaging/classes com.modernlink.messaging.JmsFacadeNatsApp
```

The facade currently supports the NATS provider path. Other provider adapters,
JNDI lookup, transactions, selectors, and durable acknowledgements remain
explicit capability gaps.

## Java 6 fixture

`java6-messaging/src` contains the Java 6 publisher and modern-provider
consumer. `LegacyJmsJmxDemo` registers a real local JMX MBean and writes one
line to stdout. `ModernProviderDemo` reads that line from stdin and validates
the shared message and trace fields.

The Java sources are compiled against the packaged `/workspace/modernlink.jar`
inside the Java 6 Docker image. A typical publisher invocation is:

```powershell
docker run --rm -v "${PWD}/hacks/java6-messaging:/workspace/hack" modernlink-java6-https sh -c "mkdir -p /workspace/hack/classes && javac -source 1.6 -target 1.6 -classpath /workspace/modernlink.jar -d /workspace/hack/classes \\$(find /workspace/hack/src -name '*.java') && java -cp /workspace/modernlink.jar:/workspace/hack/classes com.modernlink.messaging.LegacyJmsJmxDemo TRANSFORM KAFKA"
```

The fixture is intentionally small. Real JMS provider compatibility,
broker-backed delivery, acknowledgement, transactions, and external JMX
deployment remain separate backlog work in [`docs/BACKLOG.md`](../docs/BACKLOG.md).
