use crate::settings::{Modules, WidgetConfigs};
use custom_utils::{get_gif_from_path, get_image_from_path};
use gtk::prelude::*;
use relm4::{
    gtk::{
        self,
        glib::clone,
        prelude::{ButtonExt, WidgetExt},
    },
    ComponentParts, ComponentSender, SimpleComponent,
};

pub struct Settings {
    pub modules: Modules,
    pub widget_configs: WidgetConfigs,
}

pub struct NoInternet {
    settings: Settings,
}

#[derive(Debug)]
pub enum PageOutput {
    BackPressed,
    SettingsPressed,
}

pub struct AppWidgets {}

impl SimpleComponent for NoInternet {
    type Init = Settings;
    type Input = ();
    type Output = PageOutput;
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
        let model = NoInternet { settings: init };

        let main_content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(["app-container", "check-internet-text"])
            .build();

        let footer_content_box: gtk::Box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .vexpand(true)
            .valign(gtk::Align::End)
            .css_classes(["footer-container"])
            .build();

        let gif_path = modules.pages_settings.no_internet.no_internet_found.clone();
        let paintable = get_gif_from_path(gif_path);

        let image_from = gtk::Image::builder()
            .width_request(370)
            .height_request(370)
            .paintable(&paintable)
            .css_classes(["gif-img"])
            .build();

        let label1 = gtk::Label::builder()
            .label("Connect to Internet to complete the setup")
            .hexpand(true)
            .build();

        main_content_box.append(&image_from);
        main_content_box.append(&label1);

        // footer_box
        let footer_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .valign(gtk::Align::End)
            .build();

        let back_icon_img: gtk::Image = get_image_from_path(widget_configs.footer.back_icon, &[]);
        let back_button_box = gtk::Box::builder().hexpand(true).build();
        let back_button = gtk::Button::builder().build();
        back_button.set_child(Some(&back_icon_img));
        back_button.add_css_class("footer-container-button");

        back_button.connect_clicked(clone!(@strong sender => move |_| {
          let _ =  sender.output(PageOutput::BackPressed);
        }));

        let settings_icon_img: gtk::Image =
            get_image_from_path(widget_configs.footer.settings_icon, &[]);
        let settings_button = gtk::Button::new();
        settings_button.set_child(Some(&settings_icon_img));
        settings_button.add_css_class("footer-container-button");

        settings_button.connect_clicked(clone!(@strong sender => move |_| {
          let _ =  sender.output(PageOutput::SettingsPressed);
        }));

        back_button_box.append(&back_button);
        footer_box.append(&back_button_box);
        footer_box.append(&settings_button);

        footer_content_box.append(&footer_box);
        main_content_box.append(&footer_content_box);

        root.append(&main_content_box);

        let widgets = AppWidgets {};

        ComponentParts { model, widgets }
    }
}
