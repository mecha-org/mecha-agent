use async_nats::Event as NatsEvent;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum ProvisioningEvent {
    Provisioned,
    Deprovisioned,
}

#[derive(Debug, Clone)]
pub enum MessagingEvent {
    Connected,
}

#[derive(Debug, Clone)]
pub enum SettingEvent {
    Synced,
    Updated { settings: HashMap<String, String> },
}

#[derive(Debug, Clone)]
pub enum Event {
    Provisioning(ProvisioningEvent),
    Messaging(MessagingEvent),
    Settings(SettingEvent),
    Nats(NatsEvent),
}
