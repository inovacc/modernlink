# Per-provider guarantees
<!-- rev:005 (RFC 3339) 2026-08-21T00:00:00Z -->

**DOC-03**, and the documentation half of **MSG-04**. What each provider adapter in
`crates/messaging` actually offers, so a capability gap is visible *before* traffic moves
rather than discovered in production.

The machine-readable version of this table is
`Provider::guarantees()` (`crates/messaging/src/lib.rs`), reachable from Java 6 via
`ModernMessagingClient.guaranteesFor(ModernMessagingProvider)`. **This file and that
function must agree**; the function is authoritative, because it is the one the code reads.

## How to read the levels

| Level | Means |
|---|---|
| **VERIFIED** | The source table marks this behavior as backed by a recorded test run. |
| **DECLARED** | Implemented. **No test has ever exercised it.** A claim, not evidence. |
| **UNSUPPORTED** | Not offered. The helper refuses it when invoked; B-003 tracks the delivery-mode publish path that does not invoke it yet. |

Three levels rather than a boolean because "the transport implements it" and "a machine test
executed it" are different statements, and this project has been bitten by conflating them.
`Support::is_proven()` is true only for VERIFIED, deliberately.

**Most entries below are DECLARED.** Run 32386474212 at `3b64484` recorded a single configured
send → receive → acknowledge path for all five brokers. The source table remains authoritative
and has not promoted Kafka/Pulsar fields above DECLARED. Durability, reconnect, ordering under
load, concurrency, failure and redelivery are unexercised for every provider — see
[VERIFICATION.md](VERIFICATION.md) and [ISSUES.md](ISSUES.md) I-010.

## The table

| Guarantee | LEGACY_JMS | NATS | NATS_JETSTREAM | KAFKA | PULSAR | RABBITMQ |
|---|---|---|---|---|---|---|
| Persistence | UNSUPPORTED | UNSUPPORTED | DECLARED | DECLARED | DECLARED | **UNSUPPORTED** |
| Ordering | VERIFIED | DECLARED | DECLARED | DECLARED | DECLARED | DECLARED |
| Server-side ack | UNSUPPORTED | UNSUPPORTED | VERIFIED | DECLARED | DECLARED | VERIFIED |
| CLIENT ack | VERIFIED | UNSUPPORTED | VERIFIED | DECLARED | DECLARED | VERIFIED |
| Transactions | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED |
| Redelivery | UNSUPPORTED | UNSUPPORTED | DECLARED | DECLARED | DECLARED | DECLARED |
| Dead-lettering | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED |
| Replay | UNSUPPORTED | UNSUPPORTED | DECLARED | DECLARED | DECLARED | UNSUPPORTED |

## Why each column reads the way it does

### LEGACY_JMS — in-process, not a broker

`InMemoryTransport` is a `VecDeque` behind a `Mutex`. Nothing survives the process, which
is the point: it is the transparent-mode compatibility fixture, **not** a bridge to the
vendor's JMS implementation. Ordering and CLIENT acknowledgement are VERIFIED because unit
tests and `LegacyJmsMessagingTest` invoke them directly.

### NATS (core) — fire-and-forget, and the table says so

Core NATS has no broker-side state. An unacknowledged message is simply gone. The
transport's `acknowledge` returns `Acknowledged` because the *local receipt* advances, not
because a server confirmed anything — so server-side ack and CLIENT ack are UNSUPPORTED,
and requesting CLIENT acknowledgement is refused.

The dangerous answer here would not be "no". It would be a confident "yes" that silently
degrades an at-least-once contract to at-most-once.

### NATS_JETSTREAM — the strongest adapter today

A stream plus a durable pull consumer with `AckPolicy::Explicit`, so acknowledgement is
genuinely server-side and is invoked by `nats_jetstream_send_receive_ack`. Persistence,
redelivery and replay are DECLARED: JetStream provides them and the transport configures
them, but no test has restarted a broker or forced a redelivery.

### KAFKA and PULSAR — one recorded happy path; guarantees remain DECLARED

Kafka commits offsets with `CommitMode::Sync`; Pulsar acknowledges through its consumer.
Both have broker-backed tests (`crates/messaging/tests/broker_backed_{kafka,pulsar}.rs`). Run
32386474212 recorded their configured send/receive/ack jobs at `3b64484`; no restart, failure,
ordering-under-load, or redelivery path ran. `Provider::guarantees()` still marks their fields
DECLARED, and this table mirrors that source.

One caveat specific to Kafka: **ordering holds per partition, not per topic.** This
transport does not choose partitions, so ordering is only as strong as the default
partitioner makes it. A legacy application that assumes JMS queue ordering across a whole
destination will not get it.

### RABBITMQ — persistence is UNSUPPORTED, and that is a defect not a design

The queue is declared `durable: true`, but the publisher sends
`BasicProperties::default()` — AMQP `delivery_mode` 1, transient. **A durable queue holding
transient messages loses them when the broker restarts.**

The table records the behaviour rather than the intent. Recording DECLARED here because the
queue *looks* durable is precisely how a guarantee table starts lying. Tracked as
[BUGS.md](BUGS.md) **B-003**.

Server-side and CLIENT acknowledgement are VERIFIED in the source table:
`rabbitmq_send_receive_ack` invokes
the `BasicAck` path against a live broker.

## `receive()` blocks on four of six providers

The signature — `Result<Option<ReceivedMessage>>` in Rust, a nullable return in Java —
promises *"there may be no message"*. **That promise holds for two providers.**

| Provider | `receive()` when nothing is waiting |
|---|---|
| LEGACY_JMS | returns promptly with no message |
| RABBITMQ | returns promptly with no message (`basic_get` is a poll) |
| NATS | **blocks until a message arrives** |
| NATS_JETSTREAM | **blocks until a message arrives** |
| KAFKA | **blocks until a message arrives** |
| PULSAR | **blocks until a message arrives** |

For the four blocking providers `Ok(None)` is unreachable and the call simply never returns.
There is no timeout and no way to cancel, and it runs inside a JNI call the legacy
application cannot interrupt.

A JMS application polling for work expects `receiveNoWait()` semantics — call, get null, do
something else. Written against a blocking provider, that loop hangs on the first empty poll.

Query it before writing such a loop:
`ModernMessagingClient.guaranteesFor(provider).getReceiveSemantics().isSafeForPolling()`.

Making the behaviour *consistent* is a change to delivery semantics and is the maintainer's
call — tracked as [BUGS.md](BUGS.md) **B-010**. This section documents what is true today.

## Transactions and dead-lettering: nothing, everywhere

No transport implements either. Every provider therefore **refuses** `TRANSACTED`
acknowledgement rather than accepting it and behaving as if it were `AUTO` — which is
exactly how a rollback silently becomes a commit. Dead-lettering is likewise absent even
where the broker supports it (Pulsar, RabbitMQ), because this layer does not configure it.

## Timeouts

Every broker connect is bounded (H-02). Without a bound, a broker that completes the TCP
handshake and then stalls — or a firewall that DROPs rather than REJECTs — hangs the calling
thread forever, and that thread belongs to the legacy application, reached through a JNI call
it cannot cancel.

| Operation | Default |
|---|---|
| Connect / subscribe / channel open | 10s |
| Control plane: queue declare, topic creation, consumer build | 30s |

Control-plane operations get the longer default because declaring a queue or creating a topic
on a loaded cluster is legitimately slower than a handshake; a single bound tight enough to
catch a hung connect would reject healthy deployments.

Both are starting points, not policy. **`MODERNLINK_BROKER_TIMEOUT_SECS`** overrides them for
every operation. An unparseable or zero value is ignored in favour of the default — `0` would
mean "time out immediately", which presents exactly like a broker outage.

Expiry is reported as a transport failure saying the operation was *refused rather than left
to block*, and the message carries no endpoint, so it cannot become a new path for the
credential leak in [BUGS.md](BUGS.md) B-006.

## TLS — what is actually true today

**No provider connection terminates through `crates/tls`.** Only the HTTPS path does. What
each transport can do is whatever its own client library negotiates from the endpoint scheme,
and there is **no way for a caller to request TLS explicitly** — the scheme is the only
signal this API carries.

| Provider | TLS stack present in the build | How it would be selected |
|---|---|---|
| NATS / JetStream | `rustls` via `async-nats` | `tls://` or `nats+tls://` scheme |
| RabbitMQ | `rustls-connector` via `lapin` | `amqps://` scheme |
| Pulsar | `tokio-rustls` (`tokio-rustls-runtime-ring`) | `pulsar+ssl://` scheme |
| **Kafka** | **none** | **not possible — see below** |

The first three are **DECLARED, not verified**: the crates are in the dependency graph and
the schemes are the documented selectors, but no test in this repo has ever negotiated TLS
with a broker. Do not read the table as evidence.

**Kafka cannot do TLS in this build at all.** `rdkafka` is compiled with `cmake-build` and
`libz` only — there is no `ssl` feature — so librdkafka is built without OpenSSL, and
`KafkaTransport` sets only `bootstrap.servers` with no `security.protocol`. A TLS-looking
endpoint (`ssl://`, `sasl_ssl://`) is therefore **refused** rather than connected in
plaintext. Silently downgrading would leave a deployment believing its broker traffic and
credentials were encrypted, which is the worst outcome available.

Wiring real TLS — enabling `rdkafka`'s `ssl` feature, exposing TLS configuration through the
Java API, and terminating through `crates/tls` so brokers inherit the TLS 1.2 floor the HTTPS
path has — is tracked in [BACKLOG.md](BACKLOG.md).

## What is deliberately not in this table

- **A TLS *guarantee* column.** See the TLS section above for what is actually true today.
  Broker connections do not terminate through `crates/tls` — only the HTTPS path does — so a
  column in the guarantee table would imply a boundary that does not exist.
- **Backpressure.** No adapter exposes or applies one, so there is nothing to describe.

Both belong here once they are real. Adding empty columns now would suggest the analysis was
done and came back negative, which is not the same as never having been done.
