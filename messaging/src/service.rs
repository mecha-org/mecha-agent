use crate::MessagingSettings;
use mqtt::Client;
use std::thread;
use std::time::Duration;
use tonic::Code;
extern crate paho_mqtt as mqtt;

const DFLT_CLIENT: &str = "rust_publish";

#[derive(Clone)]
pub struct MessagingService {
    pub settings: MessagingSettings,
    client: Client,
}

#[derive(Debug)]
pub struct MessagingErrorResponseCode {
    pub code: Code,
    pub message: String,
}

impl MessagingService {
    pub fn new(settings: MessagingSettings) -> Self {
        let client = createMqttClient(settings.clone());
        Self {
            settings: settings,
            client: client,
        }
    }

    pub fn sendMessage(&self, topic: String, content: String) {
        let msg = mqtt::Message::new(topic, content, 0);
        println!("produce");
        let tok = &self.client.publish(msg);
        if let Err(e) = tok {
            println!("Error sending message: {:?}", e);
            try_reconnect(&self.client);
        }
    }
}

pub fn createMqttClient(settings: MessagingSettings) -> Client {
    let host = settings.mqtt.url;

    // Define the set of options for the create.
    // Use an ID for a persistent session.
    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri(host)
        .client_id(DFLT_CLIENT.to_string())
        .finalize();

    // Create a client.
    let cli = mqtt::Client::new(create_opts).unwrap();
    connect(&cli);
    cli
}

pub fn try_reconnect(cli: &mqtt::Client) -> bool {
    println!("Connection lost. Waiting to retry connection");
    for _ in 0..12 {
        thread::sleep(Duration::from_millis(5000));
        if cli.reconnect().is_ok() {
            println!("Successfully reconnected");
            return true;
        }
    }
    println!("Unable to reconnect after several attempts.");
    false
}

fn connect(client: &Client) {
    // connect(&self.client)
    println!("connect");
    let lwt = mqtt::MessageBuilder::new()
        .topic("test")
        .payload("Consumer lost connection")
        .finalize();
    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(20))
        .clean_session(false)
        .will_message(lwt)
        .finalize();
    if let Err(e) = client.connect(conn_opts) {
        println!("Unable to connect:\n\t{:?}", e);
    }
}
