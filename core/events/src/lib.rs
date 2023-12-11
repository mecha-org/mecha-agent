use tokio::sync::broadcast;

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
}
#[derive(Clone)]
pub enum Event {
    Provisioning(ProvisioningEvent),
    Messaging(MessagingEvent),
    Settings(SettingEvent),
}
