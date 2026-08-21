#!/usr/bin/env bash
set -euo pipefail

# Full-workspace Rust reporting plus a 90% production behavior-crate gate. JNI ABI glue and
# demo CLIs remain in the full report and execute through Java/live brokers, while the threshold
# excludes those adapter surfaces. Broker containers are sequential for hosted-runner capacity.

coverage_classes=""
coverage_target=""
containers=(ml-coverage-nats ml-coverage-rabbit ml-coverage-kafka ml-coverage-pulsar)

cleanup() {
  docker rm -f "${containers[@]}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

wait_for_log() {
  local container="$1"
  local pattern="$2"
  local seconds="$3"
  local deadline=$(( $(date +%s) + seconds ))
  while [ "$(date +%s)" -lt "$deadline" ]; do
    if docker logs "$container" 2>&1 | grep -q "$pattern"; then
      return 0
    fi
    sleep 2
  done
  docker logs "$container" >&2
  echo "$container did not emit readiness pattern: $pattern" >&2
  return 1
}

run_java() {
  java -cp "$coverage_classes" "$@"
}

run_standard_java_tests() {
  find "$coverage_classes/com/modernlink" -name '*Test.class' \
    ! -name 'BrokerBackedMessagingTest.class' -print | sort | while read -r class_file
  do
    class_name="${class_file#${coverage_classes}/}"
    class_name="${class_name%.class}"
    class_name="${class_name//\//.}"
    run_java "$class_name"
  done
}

run_legacy_demos() {
  cargo run -q -p messaging-demo --features all-providers --bin messaging-demo
  local frame
  frame=$(cargo run -q -p messaging-demo --features all-providers --bin legacy-jms-app \
    -- redirect kafka)
  printf '%s' "$frame" | \
    cargo run -q -p messaging-demo --features all-providers --bin modern-provider-app
}

run_nats_and_rabbit() {
  docker run -d --name ml-coverage-nats -p 4222:4222 nats:2.10 -js >/dev/null
  docker run -d --name ml-coverage-rabbit -p 5672:5672 rabbitmq:3.13 >/dev/null
  wait_for_log ml-coverage-nats 'Server is ready' 180
  wait_for_log ml-coverage-rabbit 'Server startup complete' 180

  run_java com.modernlink.messaging.BrokerBackedMessagingTest \
    NATS nats://127.0.0.1:4222 modernlink_coverage_nats
  run_java com.modernlink.messaging.BrokerBackedMessagingTest \
    NATS_JETSTREAM nats://127.0.0.1:4222 modernlink_coverage_jetstream
  run_java com.modernlink.messaging.BrokerBackedMessagingTest \
    RABBITMQ amqp://guest:guest@127.0.0.1:5672/%2f modernlink_coverage_rabbit

  MODERNLINK_NATS_URL=nats://127.0.0.1:4222 \
  MODERNLINK_RABBITMQ_URL=amqp://guest:guest@127.0.0.1:5672/%2f \
    cargo test -p messaging --test broker_backed --no-default-features \
      --features nats,rabbitmq -- --ignored --test-threads=1
  cargo test -p messaging --lib --no-default-features --features nats \
    live_coverage_nats_resource_loss_fails_closed -- --ignored --test-threads=1
  cargo test -p messaging --lib --no-default-features --features nats \
    live_coverage_jetstream_poisoned_ack_store_fails_closed -- --ignored --test-threads=1
  cargo test -p messaging --lib --no-default-features --features rabbitmq \
    live_coverage_rabbitmq_poisoned_ack_store_fails_closed -- --ignored --test-threads=1
  NATS_URL=nats://127.0.0.1:4222 \
    cargo run -q -p messaging-demo --features all-providers --bin nats-app
  NATS_URL=nats://127.0.0.1:4222 \
    cargo run -q -p messaging-demo --features all-providers --bin jetstream-app
  RABBITMQ_URI=amqp://guest:guest@127.0.0.1:5672/%2f \
    cargo run -q -p messaging-demo --features all-providers --bin rabbitmq-app

  docker rm -f ml-coverage-nats ml-coverage-rabbit >/dev/null
}

run_kafka() {
  docker run -d --name ml-coverage-kafka -p 9092:9092 \
    -e KAFKA_NODE_ID=1 \
    -e KAFKA_PROCESS_ROLES=broker,controller \
    -e KAFKA_LISTENERS=PLAINTEXT://:9092,CONTROLLER://:9093 \
    -e KAFKA_ADVERTISED_LISTENERS=PLAINTEXT://127.0.0.1:9092 \
    -e KAFKA_CONTROLLER_LISTENER_NAMES=CONTROLLER \
    -e KAFKA_LISTENER_SECURITY_PROTOCOL_MAP=CONTROLLER:PLAINTEXT,PLAINTEXT:PLAINTEXT \
    -e KAFKA_CONTROLLER_QUORUM_VOTERS=1@localhost:9093 \
    -e KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR=1 \
    -e KAFKA_TRANSACTION_STATE_LOG_REPLICATION_FACTOR=1 \
    -e KAFKA_TRANSACTION_STATE_LOG_MIN_ISR=1 \
    -e KAFKA_GROUP_INITIAL_REBALANCE_DELAY_MS=0 \
    apache/kafka:3.8.0 >/dev/null
  wait_for_log ml-coverage-kafka 'Kafka Server started' 300

  run_java com.modernlink.messaging.BrokerBackedMessagingTest \
    KAFKA 127.0.0.1:9092 modernlink_coverage_kafka
  MODERNLINK_KAFKA_BROKERS=127.0.0.1:9092 \
    cargo test -p messaging --test broker_backed_kafka --no-default-features \
      --features kafka -- --ignored --test-threads=1
  cargo test -p messaging --lib --no-default-features --features kafka \
    live_coverage_kafka_poisoned_ack_store_fails_closed -- --ignored --test-threads=1
  KAFKA_BROKERS=127.0.0.1:9092 \
    cargo run -q -p messaging-demo --features all-providers --bin kafka-app

  docker rm -f ml-coverage-kafka >/dev/null
}

run_pulsar() {
  docker run -d --name ml-coverage-pulsar -p 6650:6650 \
    -e 'PULSAR_MEM=-Xms256m -Xmx512m -XX:MaxDirectMemorySize=256m' \
    -e 'BOOKIE_MEM=-Xms256m -Xmx512m -XX:MaxDirectMemorySize=256m' \
    apachepulsar/pulsar:3.3.0 bin/pulsar standalone >/dev/null
  wait_for_log ml-coverage-pulsar 'messaging service is ready' 300

  run_java com.modernlink.messaging.BrokerBackedMessagingTest \
    PULSAR pulsar://127.0.0.1:6650 modernlink_coverage_pulsar
  MODERNLINK_PULSAR_URL=pulsar://127.0.0.1:6650 \
    cargo test -p messaging --test broker_backed_pulsar --no-default-features \
      --features pulsar -- --ignored --test-threads=1
  cargo test -p messaging --lib --no-default-features --features pulsar \
    live_coverage_pulsar_poisoned_ack_store_fails_closed -- --ignored --test-threads=1
  PULSAR_URL=pulsar://127.0.0.1:6650 \
    cargo run -q -p messaging-demo --features all-providers --bin pulsar-app

  docker rm -f ml-coverage-pulsar >/dev/null
}

# One shared instrumented target tree is required for unit tests, cargo-run demos,
# broker tests, and the native library loaded by Java. This ordering follows
# cargo-llvm-cov's external-test workflow: show-env -> clean -> normal cargo commands.
source <(cargo llvm-cov show-env --sh)
cleanup
cargo llvm-cov clean --workspace
cargo test --workspace --all-features

coverage_target="${CARGO_LLVM_COV_TARGET_DIR}"
export LLVM_PROFILE_FILE="${coverage_target}/external-%p-%8m.profraw"
cargo build -p jni-bridge --features all-providers --target-dir "$coverage_target"

coverage_classes=$(mktemp -d "${TMPDIR:-/tmp}/modernlink-coverage-java.XXXXXX")
mkdir -p "$coverage_classes/native/linux-x86_64"
find java/src/main/java -name '*.java' -print0 | \
  xargs -0 javac -source 8 -target 8 -d "$coverage_classes"
find java/src/test/java -name '*.java' -print0 | \
  xargs -0 javac -source 8 -target 8 -classpath "$coverage_classes" -d "$coverage_classes"
cp "$coverage_target/debug/libmodernlink.so" \
  "$coverage_classes/native/linux-x86_64/libmodernlink.so"

run_standard_java_tests
run_legacy_demos
run_nats_and_rabbit
run_kafka
run_pulsar

# cargo-llvm-cov excludes src/tests.rs by default. The threshold covers the four Rust
# behavior crates; JNI ABI glue and hacks/ CLI fixtures have separate end-to-end execution
# above and remain visible in the full-workspace report and HTML artifact.
coverage_scope='(crates[\\/]jni[\\/]src[\\/]lib\.rs|hacks[\\/]messaging-demo[\\/])'
echo 'Full Rust production-source coverage (informational):'
cargo llvm-cov report --summary-only
echo 'Rust behavior-crate production-source coverage (90% gate):'
cargo llvm-cov report --summary-only \
  --ignore-filename-regex "$coverage_scope" --fail-under-lines 90
cargo llvm-cov report --html --output-dir target/rust-coverage
