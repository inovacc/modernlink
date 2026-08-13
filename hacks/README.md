# ModernLink hacks

This directory contains executable compatibility fixtures. They are not part
of the distributable JAR and do not claim interoperability with an external
Kafka, Pulsar, NATS, RabbitMQ, or JMS broker.

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
