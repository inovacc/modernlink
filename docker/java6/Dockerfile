FROM rust:1.96-bookworm AS native-builder

WORKDIR /native
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN apt-get update \
    && apt-get install -y --no-install-recommends python3-pip \
    && pip3 install --break-system-packages --no-cache-dir ziglang \
    && cargo install cargo-zigbuild \
    && rustup target add x86_64-unknown-linux-gnu
RUN cargo zigbuild -p jni@0.1.0 --release --target x86_64-unknown-linux-gnu.2.13

FROM java:6b38-jdk

WORKDIR /workspace

COPY java/src /workspace/java/src
COPY --from=native-builder /native/target/x86_64-unknown-linux-gnu/release/libmodernlink.so /workspace/java/src/main/resources/native/linux-x86_64/libmodernlink.so

RUN mkdir -p /workspace/build/classes /workspace/build/classes/native/linux-x86_64 \
    && find /workspace/java/src/main/java -name '*.java' -print0 \
       | xargs -0 javac -source 1.6 -target 1.6 -d /workspace/build/classes \
    && find /workspace/java/src/test/java -name '*.java' -print0 \
       | xargs -0 javac -source 1.6 -target 1.6 \
          -classpath /workspace/build/classes \
          -d /workspace/build/classes \
    && cp /workspace/java/src/main/resources/native/linux-x86_64/libmodernlink.so \
       /workspace/build/classes/native/linux-x86_64/libmodernlink.so \
    && jar cf /workspace/modernlink.jar -C /workspace/build/classes .

CMD ["java", "-version"]
