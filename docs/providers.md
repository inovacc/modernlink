# Per-provider guarantees
<!-- rev:001 (RFC 3339) 2026-08-19T00:00:00Z -->

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
| **VERIFIED** | Implemented **and** exercised by a test that has actually run. |
| **DECLARED** | Implemented. **No test has ever exercised it.** A claim, not evidence. |
| **UNSUPPORTED** | Not offered. Requesting it is **refused**, never quietly downgraded. |

Three levels rather than a boolean because "the transport implements it" and "a test
proved it" are different statements, and this project has been bitten by conflating them.
`Support::is_proven()` is true only for VERIFIED, deliberately.

**Most entries below are DECLARED.** The only behaviour ever executed against a real
broker is a single happy-path send → receive → acknowledge round trip, on 2026-08-14, for
NATS core, JetStream and RabbitMQ. Durability, reconnect, ordering under load, concurrency,
failure and redelivery are unexercised for **every** provider — see
[BUGS.md](BUGS.md) "Verification reach" and [ISSUES.md](ISSUES.md) I-010.

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
tests and `LegacyJmsMessagingTest` exercise them directly.

### NATS (core) — fire-and-forget, and the table says so

Core NATS has no broker-side state. An unacknowledged message is simply gone. The
transport's `acknowledge` returns `Acknowledged` because the *local receipt* advances, not
because a server confirmed anything — so server-side ack and CLIENT ack are UNSUPPORTED,
and requesting CLIENT acknowledgement is refused.

The dangerous answer here would not be "no". It would be a confident "yes" that silently
degrades an at-least-once contract to at-most-once.

### NATS_JETSTREAM — the strongest adapter today

A stream plus a durable pull consumer with `AckPolicy::Explicit`, so acknowledgement is
genuinely server-side and was exercised by `nats_jetstream_send_receive_ack`. Persistence,
redelivery and replay are DECLARED: JetStream provides them and the transport configures
them, but no test has restarted a broker or forced a redelivery.

### KAFKA and PULSAR — implemented, entirely unexecuted

Kafka commits offsets with `CommitMode::Sync`; Pulsar acknowledges through its consumer.
Both have broker-backed tests as of this revision
(`crates/messaging/tests/broker_backed_{kafka,pulsar}.rs`) and **neither has ever run**, so
nothing here is above DECLARED.

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

Server-side and CLIENT acknowledgement are VERIFIED: `rabbitmq_send_receive_ack` exercises
the `BasicAck` path against a live broker.

## Transactions and dead-lettering: nothing, everywhere

No transport implements either. Every provider therefore **refuses** `TRANSACTED`
acknowledgement rather than accepting it and behaving as if it were `AUTO` — which is
exactly how a rollback silently becomes a commit. Dead-lettering is likewise absent even
where the broker supports it (Pulsar, RabbitMQ), because this layer does not configure it.

## What is deliberately not in this table

- **TLS and authentication per provider.** The broker connections are not yet TLS-terminated
  through `crates/tls`; only the HTTPS path is. Documenting a TLS column today would imply a
  boundary that does not exist.
- **Backpressure.** No adapter exposes or applies one, so there is nothing to describe.

Both belong here once they are real. Adding empty columns now would suggest the analysis was
done and came back negative, which is not the same as never having been done.
