use crate::{
    services::MachineInformation,
    settings::{Modules, WidgetConfigs},
    utils,
};
use gtk::prelude::*;
use relm4::{
    gtk::{
        self,
        glib::clone,
        pango,
        prelude::{ButtonExt, WidgetExt},
        Button,
    },
    ComponentParts, ComponentSender, SimpleComponent,
};
use utils::{get_gif_from_path, get_image_from_path};

pub struct Settings {
    pub modules: Modules,
    pub widget_configs: WidgetConfigs,
}

pub struct SetupSuccess {
    settings: Settings,
    machine_id: String,
}

#[derive(Debug)]
pub enum InputMessage {
    ActiveScreen(String),
}

#[derive(Debug)]
pub enum SetupSuccessOutput {
    BackPressed,
    NextPressed,
}

pub struct AppWidgets {}

impl SimpleComponent for SetupSuccess {
    type Init = Settings;
    type Input = InputMessage;
    type Output = SetupSuccessOutput;
    type Root = gtk::Box;
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        gtk::Box::builder().build()
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let modules = init.modules.clone();
        let widget_configs = init.widget_configs.clone();

        let model = SetupSuccess {
            settings: init,
            machine_id: String::from(""),
        };

        let main_content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(["app-container", "setup-status-label"])
            .build();

        let footer_content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .vexpand(true)
            .valign(gtk::Align::End)
            .css_classes(["footer-container"])
            .build();

        // get gif
        let gif_path = modules.pages_settings.setup_success.success.clone();
        let paintable = get_gif_from_path(gif_path);

        let image_from = gtk::Image::builder()
            .width_request(290)
            .height_request(290)
            .paintable(&paintable)
            .css_classes(["gif-img"])
            .vexpand(true)
            .valign(gtk::Align::Center)
            .build();

        // bold
        let label1: gtk::Label = gtk::Label::builder()
            .label("Machine is now connected to your Mecha account")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .justify(gtk::Justification::Center)
            .wrap(true)
            .wrap_mode(pango::WrapMode::Word)
            .build();

        main_content_box.append(&image_from);
        main_content_box.append(&label1);

        // footer_box
        let footer_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .build();
        let button_box = gtk::Box::builder().hexpand(true).build();

        let back_icon_img: gtk::Image = get_image_from_path(widget_configs.footer.back_icon, &[]);
        let back_button = Button::builder().build();
        back_button.set_child(Some(&back_icon_img));
        back_button.add_css_class("footer-container-button");

        back_button.connect_clicked(clone!(@strong sender => move |_| {
          let _ =  sender.output(SetupSuccessOutput::BackPressed);
        }));

        let next_icon_img: gtk::Image = get_image_from_path(widget_configs.footer.next_icon, &[]);
        let next_button = Button::new();
        next_button.set_child(Some(&next_icon_img));
        next_button.add_css_class("footer-container-button");

        next_button.connect_clicked(clone!(@strong sender => move |_| {
          let _ =  sender.output(SetupSuccessOutput::NextPressed);
        }));

        // button_box.append(&back_button); // remove
        footer_box.append(&button_box);
        footer_box.append(&next_button);

        footer_content_box.append(&footer_box);
        main_content_box.append(&footer_content_box);

        root.append(&main_content_box);

        let widgets = AppWidgets {};

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            InputMessage::ActiveScreen(text) => {
                tracing::info!("active screen: {:?}", text);
            }
        }
    }
}
