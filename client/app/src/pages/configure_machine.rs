use crate::{
    services::{self, MachineInformation},
    settings::{Modules, WidgetConfigs},
    utils,
};
use anyhow::{bail, Result};
use async_trait::async_trait;
use gtk::prelude::*;
use relm4::{
    component::{AsyncComponent, AsyncComponentParts},
    gtk::{
        self,
        prelude::{ButtonExt, WidgetExt},
    },
    AsyncComponentSender,
};
use std::time::Duration;
use tracing::{debug, info};
use utils::get_gif_from_path;
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

pub struct Settings {
    pub modules: Modules,
    pub widget_configs: WidgetConfigs,
}

pub struct ConfigureMachine {
    settings: Settings,
}

#[derive(Debug)]
pub enum ConfigureOutput {
    SetupSuccess(String),
    ShowError(String),
    Timeout,
}

#[derive(Debug)]
pub enum InputMessage {
    ActiveScreen(String),
}

pub struct AppWidgets {}

#[async_trait(?Send)]
impl AsyncComponent for ConfigureMachine {
    type Init = Settings;
    type Input = InputMessage;
    type Output = ConfigureOutput;
    type Root = gtk::Box;
    type Widgets = AppWidgets;
    type CommandOutput = ();

    fn init_root() -> Self::Root {
        gtk::Box::builder().build()
    }

    /// Initialize the UI and model.
    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let modules = init.modules.clone();
        let widget_configs = init.widget_configs.clone();

        let model = ConfigureMachine { settings: init };

        let main_content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(["app-container", "configure-machine-text"])
            .build();

        // get gif
        let gif_path = modules
            .pages_settings
            .configure_machine
            .machine_searching
            .clone();
        let paintable = get_gif_from_path(gif_path);

        let image_from = gtk::Image::builder()
            .width_request(290)
            .height_request(290)
            .paintable(&paintable)
            .css_classes(["gif-img"])
            .vexpand(true)
            .valign(gtk::Align::Center)
            .build();

        let label1 = gtk::Label::builder()
            .label("Fetching your machine information")
            .halign(gtk::Align::Center)
            .build();

        main_content_box.append(&image_from);
        main_content_box.append(&label1);

        let footer_content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .vexpand(true)
            .valign(gtk::Align::End)
            .css_classes(["footer-container"])
            .build();

        // footer_box
        let footer_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .build();

        footer_content_box.append(&footer_box);
        main_content_box.append(&footer_content_box);

        root.append(&main_content_box);

        let widgets = AppWidgets {};

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            InputMessage::ActiveScreen(text) => {
                info!("active screen: {:?}", text);
                let sender: AsyncComponentSender<ConfigureMachine> = sender.clone();
                let _ = get_machine_info(sender).await;
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: AsyncComponentSender<Self>) {}
}

async fn machine_info_sevice_call() -> Result<MachineInformation> {
    let result = match services::get_machine_info().await {
        Ok(res) => res,
        Err(err) => {
            bail!(err);
        }
    };
    Ok(result)
}

async fn get_machine_info(sender: AsyncComponentSender<ConfigureMachine>) {
    let fn_name = "configure_machine_screen -> get_machine_info";
    let result = tokio::time::timeout(Duration::from_secs(15), machine_info_sevice_call()).await;

    match result {
        Ok(res) => match res {
            Ok(res) => {
                let _ = sender.output(ConfigureOutput::SetupSuccess(res.machine_id));
            }
            Err(e) => {
                debug!(func = fn_name, PACKAGE_NAME, "API Error {:?}", e);
                let _ = tokio::time::sleep(Duration::from_millis(3000)).await;
                let _ = sender.output(ConfigureOutput::ShowError(e.to_string()));
            }
        },
        Err(e) => {
            debug!(
                func = fn_name,
                PACKAGE_NAME, "API Error After Timeout {:?}", e
            );
            let _ = sender.output(ConfigureOutput::Timeout);
        }
    }
}
