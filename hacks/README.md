# ModernLink hacks

This directory contains executable compatibility fixtures. They are not part
of the distributable JAR. The NATS fixtures exercise a real external NATS
broker; Kafka, RabbitMQ, and Pulsar have Cargo-backed fixtures. JMS broker
interoperability remains open.

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
java -cp hacks/java6-messaging/classes com.modernlink.messaging.JmsFacadeNatsApp nats://127.0.0.1:4222 modernlink.java6.jms.jetstream NATS_JETSTREAM REDIRECT
```

The fourth argument selects `TRANSPARENT`, `TRANSFORM`, or `REDIRECT`; it
defaults to `REDIRECT`. `LEGACY_JMS TRANSPARENT` uses the in-process
compatibility transport and is intended to exercise the legacy contract
without claiming vendor JMS-broker interoperability:

```powershell
java -cp hacks/java6-messaging/classes com.modernlink.messaging.JmsFacadeNatsApp legacy://in-process modernlink.java6.jms.facade LEGACY_JMS TRANSPARENT
```

Other provider adapters, JNDI lookup, transactions, and selectors remain
explicit capability gaps.

## Kafka-compatible broker fixture

The Cargo-backed Kafka adapter uses `rdkafka`/`librdkafka`, disables consumer
auto-commit, and commits the consumed partition offset only after the uniform
receipt is acknowledged. A local Redpanda broker can exercise it:

```powershell
docker run --rm -d --name modernlink-redpanda -p 19092:19092 `
  docker.redpanda.com/redpandadata/redpanda:v24.3.5 `
  redpanda start --overprovisioned --smp 1 --memory 1G --reserve-memory 0M `
  --node-id 0 --check=false --kafka-addr 0.0.0.0:19092 `
  --advertise-kafka-addr 127.0.0.1:19092
cargo run --manifest-path hacks/messaging-demo/Cargo.toml --bin kafka-app
docker rm -f modernlink-redpanda
```

The native Java facade accepts `KAFKA` through the same `ModernConnectionFactory`
selection used by the NATS fixtures. Kafka provider selection requires the
broker address in the URL field and derives a stable consumer group from the
destination for this compatibility fixture.

## RabbitMQ broker fixture

The Cargo-backed RabbitMQ adapter declares a durable queue, publishes the
uniform JSON envelope, reads it with `basic_get`, and acknowledges it only after
the uniform receipt is acknowledged:

```powershell
docker run --rm -d --name modernlink-rabbitmq -p 5672:5672 `
  rabbitmq:4-management-alpine
cargo run --manifest-path hacks/messaging-demo/Cargo.toml --bin rabbitmq-app
docker rm -f modernlink-rabbitmq
```

The default URI is `amqp://guest:guest@127.0.0.1:5672/%2f`; override it with
`RABBITMQ_URI` and the queue with `RABBITMQ_QUEUE`. The Java facade accepts
`RABBITMQ` through the same provider selection surface.

## Pulsar broker fixture

The Cargo-backed Pulsar adapter publishes the uniform JSON envelope to a
topic, consumes it through a subscription, and acknowledges the delivery:

```powershell
docker run --rm -d --name modernlink-pulsar -p 6650:6650 -p 8080:8080 `
  apachepulsar/pulsar:3.3.2 bin/pulsar standalone
cargo run --manifest-path hacks/messaging-demo/Cargo.toml --bin pulsar-app
docker rm -f modernlink-pulsar
```

## Java 6 fixture

`java6-messaging/src` contains the Java 6 publisher and modern-provider
consumer. `LegacyJmsJmxDemo` registers a real local JMX MBean and writes one
line to stdout. `ModernProviderDemo` reads that line from stdin and validates
the shared message and trace fields.

The Java sources are compiled against the packaged `/workspace/modernlink.jar`
inside the Java 6 Docker image. A typical publisher invocation is:

```powershell
docker run --rm -v "${PWD}/hacks/java6-messaging:/workspace/hack" modernlink-java6 sh -c "mkdir -p /workspace/hack/classes && javac -source 1.6 -target 1.6 -classpath /workspace/modernlink.jar -d /workspace/hack/classes \\$(find /workspace/hack/src -name '*.java') && java -cp /workspace/modernlink.jar:/workspace/hack/classes com.modernlink.messaging.LegacyJmsJmxDemo TRANSFORM KAFKA"
```

The fixture is intentionally small. Real JMS provider compatibility,
broker-backed delivery, acknowledgement, transactions, and external JMX
deployment remain separate backlog work in [`docs/BACKLOG.md`](../docs/BACKLOG.md).

The Java 6 distributable image also exercises the native RabbitMQ façade
directly. After building `modernlink-java6`, compile the fixture sources in the
image and run the client with an AMQP URL reachable from the container:

```powershell
docker build -f docker/java6/Dockerfile -t modernlink-java6 .
docker run --rm -v "${PWD}/hacks/java6-messaging:/workspace/hack" `
  modernlink-java6 sh -c "mkdir -p /workspace/hack/classes && javac -source 1.6 -target 1.6 -classpath /workspace/modernlink.jar -d /workspace/hack/classes \$(find /workspace/hack/src -name '*.java') && java -cp /workspace/modernlink.jar:/workspace/hack/classes com.modernlink.messaging.JmsFacadeNatsApp amqp://guest:guest@host.docker.internal:5672/%2f modernlink.java6.jms.facade RABBITMQ"
```
