FROM java:6b38-jdk

WORKDIR /workspace

# The JAR is downloaded from the GitHub release before this image is built.
# No Java or native artifact is rebuilt from the repository in this image.
COPY .release-test/modernlink.jar /workspace/modernlink.jar
COPY hacks/java6-messaging/src /workspace/fixtures-src
COPY docker/java6/release-test.sh /workspace/release-test.sh

RUN mkdir -p /workspace/build/fixtures \
    && find /workspace/fixtures-src -name '*.java' -print0 \
       | xargs -0 javac -source 1.6 -target 1.6 \
          -classpath /workspace/modernlink.jar \
          -d /workspace/build/fixtures \
    && chmod +x /workspace/release-test.sh

ENTRYPOINT ["/workspace/release-test.sh"]
