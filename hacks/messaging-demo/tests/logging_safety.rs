#[test]
fn rabbitmq_fixture_does_not_log_its_connection_uri() {
    let source = include_str!("../src/bin/rabbitmq-app.rs");
    let summary = source
        .split("println!(")
        .nth(1)
        .expect("RabbitMQ fixture should emit one operational summary")
        .split(");")
        .next()
        .expect("RabbitMQ summary macro should terminate");

    assert!(
        !summary.contains("uri={}"),
        "RabbitMQ operational output must not expose a credential-bearing URI"
    );
}
