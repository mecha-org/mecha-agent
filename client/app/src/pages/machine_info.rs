use gtk::prelude::*;
use image::codecs::webp;
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

use crate::{
    handlers::machine_info::handler::machine_status_service,
    services::MachineInformation,
    settings::{Modules, WidgetConfigs},
};
use custom_utils::{get_image_bytes, get_image_from_path, get_image_from_url};
use tonic::async_trait;

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
}

#[derive(Debug)]
enum AppInput {
    Increment,
    Decrement,
}

#[derive(Debug)]
pub enum InputMessage {
    ActiveScreen(Option<MachineInformation>),
    ShowStatus(bool),
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
            // .label("Shoaib's Compute")
            .label("".to_string())
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

        let toast_label = gtk::Label::builder()
            .label(String::from("Machine Agent not running"))
            .halign(gtk::Align::Center)
            .css_classes(["custom-toast"])
            .build();

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

        // let back_icon_img: gtk::Image = get_image_from_path(
        //    widget_configs.footer.back_icon,
        //    &[],
        // );
        // let back_button = Button::builder().build();
        // back_button.set_child(Some(&back_icon_img));
        // back_button.add_css_class("footer-container-button");

        // back_button.connect_clicked(clone!(@strong sender => move |_| {
        //     let _ =  sender.output(DevicePageOutput::BackPressed);
        //   }));

        // let trash_icon_img: gtk::Image = get_image_from_path(
        //     widget_configs.footer.trash_icon,
        //     &[],
        // );
        // let trash_button = Button::new();
        // trash_button.set_child(Some(&trash_icon_img));
        // trash_button.add_css_class("footer-container-button");

        // trash_button.connect_clicked(clone!(@strong sender => move |_| {
        //     let _ =  sender.output(DevicePageOutput::NextPressed);
        //   }));
        // footer_box.append(&trash_button);

        footer_box.append(&button_box);
        footer_box.append(&exit_button);

        footer_content_box.append(&footer_box);
        main_content_box.append(&footer_content_box);

        root.append(&main_content_box);

        let model = MachineInfo {
            settings: init,
            machine_id: String::from(""),
            name: String::from(""),
            icon_path: Some(String::from("")),
            status: false,
            icon_bytes: None,
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
            InputMessage::ActiveScreen(response) => {
                if let Some(response) = &response {
                    self.machine_id = response.machine_id.to_owned();
                    self.name = response.name.to_owned();
                    self.icon_path = response.icon.to_owned();
                    self.icon_bytes = get_image_bytes(self.icon_path.clone()).await;
                };

                let sender: relm4::Sender<InputMessage> = sender.input_sender().clone();
                _ = init_services(sender).await;
            }
            InputMessage::ShowStatus(status) => {
                self.status = status;
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: AsyncComponentSender<Self>) {
        widgets.name_label.set_label(&self.name);
        widgets.id_label.set_label(&self.machine_id);

        if self.status == true {
            if let Some(bytes) = self.icon_bytes.clone() {
                let paintable = get_image_from_url(Some(bytes), &["device-info-status-icon"]);
                widgets.profile_icon.set_paintable(Some(&paintable));
                widgets
                    .profile_icon
                    .style_context()
                    .add_class("device-info-icon");
            }
        } else {
            widgets.not_active_status_icon.set_visible(true);
            widgets.active_status_icon.set_visible(false);

            widgets.toast_label.set_label(&String::from(
                "Machine Agent not running or not internet connectivity",
            ));
            widgets.toast_label.set_visible(true);
            widgets.toast_label.set_hexpand(true);
        }
        widgets.active_status_icon.set_visible(true);
        widgets.not_active_status_icon.set_visible(false);

        widgets.toast_label.set_visible(false);
    }
}

async fn init_services(sender: relm4::Sender<InputMessage>) {
    let sender_clone_1 = sender.clone();
    let _ = relm4::spawn(async move {
        let _ = machine_status_service(sender_clone_1).await;
    });
}
