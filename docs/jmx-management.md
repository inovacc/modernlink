# JMX management surface

`ModernConnection` registers a read-only standard MBean when it is created and
unregisters it when the connection closes. The object name is:

```text
com.modernlink.messaging:type=Metrics,mode=<MODE>,provider=<PROVIDER>,connection=<ID>
```

The MBean exposes only mode, provider, publish/receive/acknowledgement counts,
and the last trace ID. It does not expose payloads, destinations, credentials,
or transport configuration. The connection ID is scoped to the JVM process;
monitoring should discover instances by the stable domain/type/mode/provider
keys rather than persist the numeric ID.

The standalone Java 6 publisher fixture has no `ModernConnection`, so it
continues to register its own publisher MBean for the process-to-process demo.
This is a fixture path, not a replacement for vendor-server JMX integration.
