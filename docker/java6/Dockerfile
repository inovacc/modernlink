FROM java:6b38-jdk

WORKDIR /workspace

COPY java/src /workspace/java/src

RUN mkdir -p /workspace/build/classes \
    && find /workspace/java/src/main/java -name '*.java' -print0 \
       | xargs -0 javac -source 1.6 -target 1.6 -d /workspace/build/classes \
    && find /workspace/java/src/test/java -name '*.java' -print0 \
       | xargs -0 javac -source 1.6 -target 1.6 \
          -classpath /workspace/build/classes \
          -d /workspace/build/classes

CMD ["java", "-version"]
