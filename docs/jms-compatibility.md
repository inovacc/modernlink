# JMS compatibility boundary

The current public Java 6 surface is a source-compatible façade under
`com.modernlink.messaging`. It is not a binary-compatible replacement for
`javax.jms`; a vendor adapter is still required when the application server
expects a specific JMS provider implementation.

| Legacy concept | ModernLink surface | Current semantic |
| --- | --- | --- |
| `ConnectionFactory` | `ModernConnectionFactory` | Stores URL, destination, mode, and provider; creates a native-backed connection. |
| `Connection` | `ModernConnection` | Lifecycle, `start`, session creation, close, and read-only JMX metrics. |
| `Session` | `ModernSession` | Producer/consumer creation, text-message creation, and acknowledgement mode. |
| `MessageProducer` | `ModernMessageProducer` | Synchronous send to one destination; returns a typed delivery receipt. |
| `MessageConsumer` | `ModernMessageConsumer` | Synchronous receive, optional listener start, and explicit acknowledgement. |
| `TextMessage` | `ModernTextMessage` | UTF-8 text payload plus provider-neutral message metadata. |
| `MessageListener` | `ModernMessageListener` | Java 6 callback shape; listener execution is controlled by `Connection.start()` and continues until consumer close. |
| JMS acknowledgement | `ModernAcknowledgementMode` | `AUTO`, `CLIENT`, and `DUPS_OK` are represented in the envelope boundary. |
| JMS tracing headers | `ModernTraceContext` | Trace ID, span ID, parent span, trace state, and sampled flag are typed fields. |

Transparent mode remains an integration gap: the native transport selection
currently rejects `LEGACY_JMS`, so an application-server-specific JMS bridge
must be added before existing vendor connections can be intercepted without
source changes. Transactions, selectors, rollback/redelivery, and
dead-letter semantics are intentionally not claimed by this façade yet.

The Java 6 distributable must keep these classes free of Java 8 language and
library requirements. Provider clients stay behind the native boundary so the
legacy application loads only the façade and the platform-selected native
resource from the JAR.
