use std::collections::HashMap;

#[derive(Clone)]
pub enum ProvisioningEvent {
    Provisioned,
    Deprovisioned,
}

#[derive(Clone)]
pub enum MessagingEvent {
    Connected,
}

#[derive(Clone)]
pub enum SettingEvent {
    Synced,
    Updated { settings: HashMap<String, String> },
}
#[derive(Clone)]
pub enum Event {
    Provisioning(ProvisioningEvent),
    Messaging(MessagingEvent),
    Settings(SettingEvent),
}
