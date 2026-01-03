use std::time::Duration;

use certgenutil::{
    generate_self_signed_cert, generate_server_cert_by_ca_pem, get_cert_pem, get_key_pem,
};
use rumqttc::v5::{AsyncClient, Event, Incoming, MqttOptions, mqttbytes::QoS};
use rumqttd::{Broker, Config};

use mqtt_auth_proxy::config as mqttconfig;

#[tokio::test]
pub async fn e2e() {
    // start broker with TLS
    prepare_certs();
    start_broker();
    // configure and start proxy
    let config = mqttconfig::Config {
        remote: mqttconfig::Remote {
            host: "localhost".to_owned(),
            port: 8883,
        },
        listener: Some("127.0.0.1:1884".to_owned()),
        tls: Some(mqttconfig::Tls {
            enabled: true,
            ca_cert: Some("e2e.ca.cert.pem".into()),
        }),
        auth: Some(mqttconfig::Auth {
            auth: mqttconfig::AuthMethod::BasicAuth {
                username: "dummy".to_owned(),
                password: "dummy".to_owned(),
            },
        }),
    };
    tokio::spawn(mqtt_auth_proxy::run_proxy(config));
    // connect via proxy and test connection
    client().await;
}

fn start_broker() {
    let config = config::Config::builder()
        .add_source(config::File::with_name("tests/rumqttd.toml"))
        .build()
        .unwrap();

    let config: Config = config.try_deserialize().unwrap();
    let mut broker = Broker::new(config);
    std::thread::spawn(move || {
        broker.start().unwrap();
    });
}

async fn client() {
    let mut mqttoptions = MqttOptions::new("proxy-e2e", "localhost", 1884);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    client.subscribe("hello", QoS::AtLeastOnce).await.unwrap();
    client
        .publish("hello", QoS::AtLeastOnce, false, "world".as_bytes())
        .await
        .unwrap();
    while let Ok(notification) = eventloop.poll().await {
        if let Event::Incoming(Incoming::Publish(publish)) = notification {
            assert_eq!(publish.payload, "world".as_bytes());
            return;
        };
    }
    panic!("Error during mqtt connection");
}

fn prepare_certs() {
    // check if ca cert exists
    let ca_cert_path = std::path::Path::new("e2e.ca.cert.pem");
    let ca_key_path = std::path::Path::new("e2e.ca.key.pem");
    if !ca_cert_path.exists() || !ca_key_path.exists() {
        let (cert, private_key) = generate_self_signed_cert("e2e.ca", true, 365, vec![]).unwrap();
        std::fs::write(ca_cert_path, get_cert_pem(cert.first().unwrap())).unwrap();
        std::fs::write(ca_key_path, get_key_pem(&private_key).unwrap()).unwrap();
    }
    let server_cert_path = std::path::Path::new("e2e.server.cert.pem");
    let server_key_path = std::path::Path::new("e2e.server.key.pem");
    if !server_cert_path.exists() || !server_key_path.exists() {
        let mut ca_cert = std::fs::read_to_string(ca_cert_path).unwrap();
        let ca_key = std::fs::read_to_string(ca_key_path).unwrap();
        ca_cert.push_str(&ca_key);
        let (cert, private_key) = generate_server_cert_by_ca_pem(
            ca_cert,
            "localhost",
            365,
            vec!["localhost".to_string()],
        )
        .unwrap();
        std::fs::write(server_cert_path, get_cert_pem(cert.first().unwrap())).unwrap();
        std::fs::write(server_key_path, get_key_pem(&private_key).unwrap()).unwrap();
    }
}
