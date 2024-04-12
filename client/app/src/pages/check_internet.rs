use crate::{
    handlers::machine_info::handler::get_status,
    settings::{Modules, WidgetConfigs},
};
use async_trait::async_trait;
use custom_utils::{get_gif_from_path, get_image_from_path};
use gtk::prelude::*;
use relm4::{
    component::{AsyncComponent, AsyncComponentParts},
    gtk::{
        self,
        glib::clone,
        prelude::{ButtonExt, WidgetExt},
        Button,
    },
    AsyncComponentSender,
};
use std::time::Duration;
use tracing::{error, info, trace};

pub struct Settings {
    pub modules: Modules,
    pub widget_configs: WidgetConfigs,
}
pub struct CheckInternet {
    settings: Settings,
    task: Option<relm4::prelude::adw::glib::JoinHandle<()>>,
}
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug)]
enum AppInput {}

#[derive(Debug)]
pub enum InputMessage {
    ActiveScreen(String),
    NextScreen,
    ConnectionNotFound,
    ShowError(String),
    BackScreen,
}

#[derive(Debug)]
pub enum CheckInternetOutput {
    BackPressed,
    LinkMachine,
    ConnectionNotFound,
    ShowError(String),
}

pub struct AppWidgets {}

#[async_trait(?Send)]
impl AsyncComponent for CheckInternet {
    type Init = Settings;
    type Input = InputMessage;
    type Output = CheckInternetOutput;
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

        let model = CheckInternet {
            settings: init,
            task: None,
        };

        let main_content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(["app-container", "check-internet-text"])
            .build();

        let footer_content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .vexpand(true)
            .valign(gtk::Align::End)
            .css_classes(["footer-container"])
            .build();

        // get gif
        let gif_path = modules.pages_settings.check_internet.search_wifi.clone();
        let paintable = get_gif_from_path(gif_path);

        let image_from = gtk::Image::builder()
            .width_request(370)
            .height_request(370)
            .paintable(&paintable)
            .css_classes(["gif-img"])
            .build();

        let label1: gtk::Label = gtk::Label::builder()
            .label("Checking for internet connectivity ...")
            .build();

        main_content_box.append(&image_from);
        main_content_box.append(&label1);

        let footer_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .build();

        let back_icon_img: gtk::Image = get_image_from_path(widget_configs.footer.back_icon, &[]);
        let back_button_box = gtk::Box::builder().hexpand(true).build();
        let back_button = Button::builder().build();
        back_button.set_child(Some(&back_icon_img));
        back_button.add_css_class("footer-container-button");

        back_button.connect_clicked(clone!(@strong sender => move |_| {
            let _ =  sender.input_sender().send(InputMessage::BackScreen);
        }));

        back_button_box.append(&back_button);
        footer_box.append(&back_button_box);

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
                let sender: relm4::Sender<InputMessage> = sender.input_sender().clone();
                let relm_task: relm4::prelude::adw::glib::JoinHandle<()> =
                    relm4::spawn_local(async move {
                        let time_duration = Duration::from_millis(7000);
                        let _ = tokio::time::sleep(time_duration).await;
                        let _ = check_internet_init_services(sender).await;
                    });

                self.task = Some(relm_task);
            }
            InputMessage::BackScreen => {
                self.task.as_ref().unwrap().abort();
                let _ = sender.output(CheckInternetOutput::BackPressed);
            }
            InputMessage::NextScreen => {
                let _ = sender.output(CheckInternetOutput::LinkMachine);
            }
            InputMessage::ConnectionNotFound => {
                let _ = sender.output(CheckInternetOutput::ConnectionNotFound);
            }
            InputMessage::ShowError(text) => {
                let _ = sender.output(CheckInternetOutput::ShowError(text));
            }
        }
    }
}

async fn check_internet_init_services(sender: relm4::Sender<InputMessage>) {
    let fn_name = "check_internet_init_services";
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "Check Internet Screen - ping"
    );

    match get_status().await {
        Ok(response) => {
            if response.code == "success" {
                trace!(
                    fn_name,
                    package = PACKAGE_NAME,
                    "ping success ==> moving to Link Machine Screen"
                );
                let _ = sender.send(InputMessage::NextScreen);
            } else {
                trace!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "ping false ==> moving to No Internet Connection Screen",
                );
                let _ = sender.send(InputMessage::ConnectionNotFound);
            }
        }
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "Error in checking ping {:?}",
                e
            );
            let _ = sender.send(InputMessage::ShowError(e.to_string()));
        }
    }
}
