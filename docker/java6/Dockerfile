FROM rust:1.96-bookworm AS native-builder

WORKDIR /native
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY hacks/messaging-demo ./hacks/messaging-demo
RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake libcurl4-openssl-dev protobuf-compiler python3-pip \
    && pip3 install --break-system-packages --no-cache-dir ziglang \
    && cargo install cargo-zigbuild \
    && rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-pc-windows-gnu
ENV CFLAGS="-idirafter /usr/include/x86_64-linux-gnu -idirafter /usr/include -UWITH_OAUTHBEARER_OIDC" \
    CXXFLAGS="-UWITH_OAUTHBEARER_OIDC"
RUN cargo zigbuild -p jni@0.1.0 --release --target x86_64-unknown-linux-gnu.2.13 \
    && cargo zigbuild -p jni@0.1.0 --release --target aarch64-unknown-linux-gnu.2.17 \
    && cargo zigbuild -p jni@0.1.0 --release --target x86_64-pc-windows-gnu

FROM java:6b38-jdk

WORKDIR /workspace

COPY java/src /workspace/java/src
COPY hacks/java6-messaging/src /workspace/hacks/java6-messaging/src
COPY --from=native-builder /native/target/x86_64-unknown-linux-gnu/release/libmodernlink.so /workspace/java/src/main/resources/native/linux-x86_64/libmodernlink.so
COPY --from=native-builder /native/target/aarch64-unknown-linux-gnu/release/libmodernlink.so /workspace/java/src/main/resources/native/linux-aarch64/libmodernlink.so
COPY --from=native-builder /native/target/x86_64-pc-windows-gnu/release/modernlink.dll /workspace/java/src/main/resources/native/windows-x86_64/modernlink.dll

RUN mkdir -p /workspace/build/classes /workspace/build/classes/native/linux-x86_64 /workspace/build/classes/native/linux-aarch64 /workspace/build/classes/native/windows-x86_64 \
    && find /workspace/java/src/main/java -name '*.java' -print0 \
       | xargs -0 javac -source 1.6 -target 1.6 -d /workspace/build/classes \
    && find /workspace/java/src/test/java -name '*.java' -print0 \
       | xargs -0 javac -source 1.6 -target 1.6 \
          -classpath /workspace/build/classes \
          -d /workspace/build/classes \
    && cp /workspace/java/src/main/resources/native/linux-x86_64/libmodernlink.so \
       /workspace/build/classes/native/linux-x86_64/libmodernlink.so \
    && cp /workspace/java/src/main/resources/native/linux-aarch64/libmodernlink.so \
       /workspace/build/classes/native/linux-aarch64/libmodernlink.so \
    && cp /workspace/java/src/main/resources/native/windows-x86_64/modernlink.dll \
       /workspace/build/classes/native/windows-x86_64/modernlink.dll \
    && jar cf /workspace/modernlink.jar -C /workspace/build/classes .

# VER-07: the hacks/ contract fixtures were in no build path at all, so nothing
# checked they still compiled — let alone under the Java 6 compiler that judges
# everything else. Build them here, but into a SEPARATE output tree so demo classes
# stay out of the distributable JAR.
RUN mkdir -p /workspace/build/fixtures \
    && find /workspace/hacks/java6-messaging/src -name '*.java' -print0 \
       | xargs -0 javac -source 1.6 -target 1.6 \
          -classpath /workspace/modernlink.jar \
          -d /workspace/build/fixtures

CMD ["java", "-version"]
