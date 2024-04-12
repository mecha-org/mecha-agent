use std::time::Duration;

use crate::{
    handlers::machine_info::handler::get_status,
    services::{get_machine_info, MachineInformation},
    settings::{Modules, WidgetConfigs},
};
use custom_utils::{get_image_bytes, get_image_from_path, get_image_from_url};
use gtk::prelude::*;
use relm4::{
    component::{AsyncComponent, AsyncComponentParts},
    gtk::{
        self,
        glib::clone,
        pango,
        prelude::{ButtonExt, WidgetExt},
        Button,
    },
    AsyncComponentSender,
};
use tonic::async_trait;
use tracing::{debug, error, info};
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

pub struct Settings {
    pub modules: Modules,
    pub widget_configs: WidgetConfigs,
}

pub struct MachineInfo {
    settings: Settings,
    machine_id: String,
    name: String,
    icon_path: Option<String>,
    status: bool,
    icon_bytes: Option<relm4::gtk::glib::Bytes>,
    toast_text: String, // machine_info: Option<MachineInformation>,
}

#[derive(Debug)]
pub enum InputMessage {
    ActiveScreen(String),
    ShowStatus(bool, String),
    UpdateMachineInfo(MachineInformation),
}

#[derive(Debug)]
pub enum DevicePageOutput {
    Exit,
}

pub struct AppWidgets {
    name_label: gtk::Label,
    id_label: gtk::Label,
    profile_icon: gtk::Image,
    active_status_icon: gtk::Image,
    not_active_status_icon: gtk::Image,
    toast_label: gtk::Label,
}

#[async_trait(?Send)]
impl AsyncComponent for MachineInfo {
    type Init = Settings;
    type Input = InputMessage;
    type Output = DevicePageOutput;
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

        let main_content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(["app-container"])
            .halign(gtk::Align::Fill)
            .build();

        let footer_content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .vexpand(true)
            .valign(gtk::Align::End)
            .css_classes(["footer-container"])
            .build();

        // let user_profile_icon = gtk::Image::new();
        let user_profile_icon: gtk::Image = get_image_from_path(
            modules.pages_settings.device_info.user_profile_img.clone(),
            &["device-info-icon"],
        );

        main_content_box.append(&user_profile_icon);

        let status_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .build();

        // bold
        let machine_name: gtk::Label = gtk::Label::builder()
            .label("My Machine".to_string())
            .halign(gtk::Align::Center)
            .css_classes(["about-device-name"])
            .build();

        let active_status_icon: gtk::Image = get_image_from_path(
            modules
                .pages_settings
                .device_info
                .active_status_icon
                .clone(),
            &["device-info-status-icon"],
        );
        active_status_icon.set_visible(false);

        let not_active_status_icon: gtk::Image = get_image_from_path(
            modules
                .pages_settings
                .device_info
                .not_active_status_icon
                .clone(),
            &["device-info-status-icon"],
        );
        not_active_status_icon.set_visible(false);

        status_box.append(&machine_name);
        status_box.append(&active_status_icon);
        status_box.append(&not_active_status_icon);

        main_content_box.append(&status_box);

        let id_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .css_classes(["device-info-border-box"])
            .build();

        let id_label: gtk::Label = gtk::Label::builder()
            .label("Machine ID")
            .hexpand(true)
            .halign(gtk::Align::Start)
            .css_classes(["device-id-text", "about-device-id"])
            .build();

        let id_value: gtk::Label = gtk::Label::builder()
            .label("-")
            .halign(gtk::Align::End)
            .css_classes(["about-device-id"])
            .build();

        id_box.append(&id_label);
        id_box.append(&id_value);
        main_content_box.append(&id_box);

        let sentence_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .halign(gtk::Align::Start)
            .css_classes(["device-info-sentence"])
            .build();

        let sentence: gtk::Label = gtk::Label::builder()
            .label("You can unlink your machine from your Mecha account")
            .wrap(true)
            .wrap_mode(pango::WrapMode::Word)
            .hexpand(true)
            .build();

        sentence_box.append(&sentence);
        main_content_box.append(&sentence_box);

        let toast_text = String::from("Fetching Machine Info...");

        let toast_label = gtk::Label::builder()
            .label(toast_text.to_owned())
            .halign(gtk::Align::Center)
            .css_classes(["custom-toast"])
            .build();
        toast_label.set_visible(true);

        let toast_overlay = gtk::Overlay::builder().build();
        toast_overlay.add_overlay(&toast_label);
        toast_overlay.set_accessible_role(gtk::AccessibleRole::Generic);
        main_content_box.append(&toast_overlay);

        let footer_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .vexpand(true)
            .build();

        let button_box = gtk::Box::builder().hexpand(true).build();

        let exit_icon_img: gtk::Image = get_image_from_path(widget_configs.footer.exit_icon, &[]);

        let exit_button = Button::builder().build();
        exit_button.set_child(Some(&exit_icon_img));
        exit_button.add_css_class("footer-container-button");

        exit_button.connect_clicked(clone!(@strong sender => move |_| {
          let _ =  sender.output(DevicePageOutput::Exit);
        }));

        footer_box.append(&button_box);
        footer_box.append(&exit_button);

        footer_content_box.append(&footer_box);
        main_content_box.append(&footer_content_box);

        root.append(&main_content_box);

        let model = MachineInfo {
            settings: init,
            machine_id: String::from("-"),
            name: String::from("My Machine"),
            icon_path: Some(String::from("")),
            status: false,
            icon_bytes: None,
            toast_text: toast_text,
        };

        let widgets = AppWidgets {
            name_label: machine_name,
            id_label: id_value,
            profile_icon: user_profile_icon,
            active_status_icon: active_status_icon,
            not_active_status_icon: not_active_status_icon,
            toast_label: toast_label,
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            InputMessage::ActiveScreen(machine_id) => {
                info!("active screen: {:?}", machine_id);
                let sender: AsyncComponentSender<MachineInfo> = sender.clone();

                self.machine_id = machine_id.to_owned();
                self.toast_text = String::from("Fetching Machine Information...");

                let _ = machine_info_init_services(sender).await;
            }
            InputMessage::ShowStatus(status, error_toast) => {
                self.status = status;
                self.toast_text = error_toast;
            }
            InputMessage::UpdateMachineInfo(data) => {
                self.machine_id = data.machine_id;
                self.name = data.name;
                self.icon_path = data.icon;
                self.icon_bytes = get_image_bytes(self.icon_path.clone()).await;
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: AsyncComponentSender<Self>) {
        widgets.name_label.set_label(&self.name);
        widgets.id_label.set_label(&self.machine_id);
        widgets.active_status_icon.set_visible(false);
        widgets.not_active_status_icon.set_visible(true);

        if self.status == true {
            if let Some(bytes) = self.icon_bytes.clone() {
                let paintable = get_image_from_url(Some(bytes), &["device-info-status-icon"]);
                widgets.profile_icon.set_paintable(Some(&paintable));
                widgets
                    .profile_icon
                    .style_context()
                    .add_class("device-info-icon");
            }
            widgets.active_status_icon.set_visible(true);
            widgets.not_active_status_icon.set_visible(false);
            widgets.toast_label.set_visible(false);
        } else {
            widgets.not_active_status_icon.set_visible(true);
            widgets.active_status_icon.set_visible(false);

            widgets.toast_label.set_label(&self.toast_text.clone());
            // widgets.toast_label.set_label(&String::from(
            //     "Machine Agent not running or not internet connectivity",
            // ));
            widgets.toast_label.set_visible(true);
            widgets.toast_label.set_hexpand(true);
        }
    }
}

async fn machine_info_init_services(sender: AsyncComponentSender<MachineInfo>) {
    let fn_name = "machine_info_init_services";
    let error_toast = String::from("Machine Agent not running or not internet connectivity");
    info!(func = fn_name, package = PACKAGE_NAME);

    let _ = relm4::spawn(async move {
        loop {
            let _ = tokio::time::sleep(Duration::from_secs(10)).await;

            let (get_status_result, get_machine_info_result) =
                tokio::join!(get_status(), get_machine_info());

            match (get_status_result, get_machine_info_result) {
                (Ok(ping_res), Ok(machine_info_res)) => {
                    info!(
                        func = fn_name,
                        package = PACKAGE_NAME,
                        "MACHINE INFO IS {:?}",
                        machine_info_res.clone()
                    );

                    let _ = sender.input_sender().send(InputMessage::ShowStatus(
                        ping_res.code == "success",
                        String::from(""),
                    ));

                    let _ = sender
                        .input_sender()
                        .send(InputMessage::UpdateMachineInfo(machine_info_res.to_owned()));
                }
                (Ok(_), Err(ping_error)) => {
                    debug!(
                        func = fn_name,
                        package = PACKAGE_NAME,
                        "PING STATUS ERROR {:?}",
                        ping_error
                    );
                    let _ = sender
                        .input_sender()
                        .send(InputMessage::ShowStatus(false, error_toast.to_owned()));
                }
                (Err(machine_info_error), Ok(_)) => {
                    debug!(
                        func = fn_name,
                        package = PACKAGE_NAME,
                        "MACHINE INFO ERROR {:?}",
                        machine_info_error
                    );

                    let _ = sender
                        .input_sender()
                        .send(InputMessage::ShowStatus(false, error_toast.to_owned()));
                }
                (Err(err_1), Err(err_2)) => {
                    debug!(
                        func = fn_name,
                        package = PACKAGE_NAME,
                        "DEBUG ERR_1 {:?} & ERR_2 {:?}",
                        err_1,
                        err_2
                    );

                    error!(
                        func = fn_name,
                        package = PACKAGE_NAME,
                        "error ===> ERR_1 {:?} & ERR_2 {:?}",
                        err_1,
                        err_2
                    );

                    let _ = sender
                        .input_sender()
                        .send(InputMessage::ShowStatus(false, error_toast.to_owned()));
                }
            }
        }
    });
}
