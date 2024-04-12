use crate::settings::{Modules, WidgetConfigs};
use custom_utils::get_image_from_path;
use gtk::prelude::*;
use relm4::{
    gtk::{
        self,
        glib::clone,
        prelude::{ButtonExt, WidgetExt},
        Button,
    },
    ComponentParts, ComponentSender, SimpleComponent,
};

pub struct Settings {
    pub modules: Modules,
    pub widget_configs: WidgetConfigs,
}

pub struct TimeoutScreen {
    settings: Settings,
}

#[derive(Debug)]
pub enum TimeoutOutput {
    refreshPressed,
    BackPressed, // tmep
}

pub struct AppWidgets {}

impl SimpleComponent for TimeoutScreen {
    type Init = Settings;
    type Input = ();
    type Output = TimeoutOutput;
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
        let model = TimeoutScreen { settings: init };

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

        let image_path: Option<String> =
            modules.pages_settings.timeout_screen.timeout_image.clone();
        let timeout_image: gtk::Image = get_image_from_path(image_path, &["timeout-img"]);

        let label1 = gtk::Label::builder()
            .label("Request timed out, please try again")
            .hexpand(true)
            .build();

        main_content_box.append(&timeout_image);
        main_content_box.append(&label1);

        // footer_box
        let footer_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .valign(gtk::Align::End)
            .build();

        let refresh_icon_img: gtk::Image =
            get_image_from_path(widget_configs.footer.refresh_icon, &[]);
        let refresh_button = Button::new();
        refresh_button.set_child(Some(&refresh_icon_img));
        refresh_button.add_css_class("footer-container-button");

        refresh_button.connect_clicked(clone!(@strong sender => move |_| {
        //   let _ =  sender.output(SetupFailOutput::NextPressed);
          let _ =  sender.output(TimeoutOutput::refreshPressed);
        }));
        let button_box = gtk::Box::builder().hexpand(true).build();

        let back_icon_img: gtk::Image = get_image_from_path(widget_configs.footer.back_icon, &[]);
        let back_button = Button::builder().build();
        back_button.set_child(Some(&back_icon_img));
        back_button.add_css_class("footer-container-button");

        back_button.connect_clicked(clone!(@strong sender => move |_| {
          let _ =  sender.output(TimeoutOutput::BackPressed);
        }));

        // button_box.append(&back_button);  // temp - remove later
        footer_box.append(&button_box);
        footer_box.append(&refresh_button);

        footer_content_box.append(&footer_box);
        main_content_box.append(&footer_content_box);

        root.append(&main_content_box);

        let widgets = AppWidgets {};

        ComponentParts { model, widgets }
    }
}
