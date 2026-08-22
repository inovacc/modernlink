#!/bin/sh
set -eu

JAR=/workspace/modernlink.jar
FIXTURES=/workspace/build/fixtures

echo "release-jar=$(wc -c < "$JAR") bytes"

# The distributable JAR contains the standalone Java probes. Exclude the one
# parameterized broker probe here; it is invoked below with the live endpoint.
jar tf "$JAR" \
  | awk '/^com\/.*Test\.class$/ && $0 !~ /BrokerBackedMessagingTest\.class$/ {
      sub(/\.class$/, "", $0); gsub(/\//, ".", $0); print
    }' \
  | sort \
  | while read class_name
    do
      echo "==== $class_name"
      java -cp "$JAR" "$class_name"
    done

echo "==== com.modernlink.messaging.BrokerBackedMessagingTest NATS"
java -cp "$JAR" \
  com.modernlink.messaging.BrokerBackedMessagingTest \
  NATS nats://127.0.0.1:4222 modernlink_release_nats

echo "==== com.modernlink.messaging.LegacyJmsJmxDemo"
java -cp "$JAR:$FIXTURES" \
  com.modernlink.messaging.LegacyJmsJmxDemo TRANSPARENT LEGACY_JMS

echo "release-jar-tests=complete"
