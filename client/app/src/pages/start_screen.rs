use crate::{
    handlers::start_screen::handler::machine_provision_status,
    settings::{Modules, WidgetConfigs},
};
use custom_utils::get_image_from_path;
use gtk::prelude::*;
use relm4::{
    adw,
    component::{AsyncComponent, AsyncComponentParts},
    gtk::{
        self,
        glib::clone,
        pango,
        prelude::{ButtonExt, StyleContextExt, WidgetExt},
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
pub struct StartScreen {
    settings: Settings,
}

#[derive(Debug)]
pub enum StartScreenOutput {
    ShowCheckInternet,
    ShowMachineInfo,
    BackPressed,
}

pub struct AppWidgets {}

#[async_trait(?Send)]
impl AsyncComponent for StartScreen {
    type Init = Settings;
    type Input = ();
    type Output = StartScreenOutput;
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

        let model = StartScreen { settings: init };

        let main_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let main_content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(["app-container"])
            .build();

        let footer_content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .vexpand(true)
            .valign(gtk::Align::End)
            .css_classes(["footer-container"])
            .build();

        let header_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .css_classes(["start-screen-header-box"])
            .build();

        let app_icon_path: Option<String> = modules.pages_settings.start_screen.app_icon.clone();

        let app_icon: gtk::Image = get_image_from_path(app_icon_path, &["app-icon"]);

        let header_label = gtk::Label::builder()
            .label("Connect to Mecha")
            .halign(gtk::Align::Start)
            .build();

        header_label
            .style_context()
            .add_class("start-screen-header");

        header_box.append(&app_icon);
        header_box.append(&header_label);

        main_container.append(&header_box); // main box

        let info_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .css_classes(["start-header-p"])
            .build();

        let info_icon: gtk::Image = get_image_from_path(
            modules.pages_settings.start_screen.info_icon.clone(),
            &["info-icon"],
        );

        let info_sentence = gtk::Label::builder()
            .label("Please sign up on mecha.so before getting started.")
            .halign(gtk::Align::Start)
            .build();

        info_box.append(&info_icon);
        info_box.append(&info_sentence);
        main_content_box.append(&info_box);

        let hbox_line2 = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .css_classes(["start-screen-steps-box"])
            .build();

        let icon2: gtk::Image = get_image_from_path(
            modules
                .pages_settings
                .start_screen
                .virtual_network_icon
                .clone(),
            &["start-screen-steps-icon"],
        );

        let label2 = gtk::Label::builder()
            .label("Mesh Networking to enable global connectivity between your machines")
            .css_classes(["start-screen-steps-label"])
            .wrap(true)
            .wrap_mode(pango::WrapMode::Word)
            .vexpand(true)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::End)
            .justify(gtk::Justification::Center)
            .build();

        hbox_line2.append(&icon2);
        hbox_line2.append(&label2);

        let hbox_line3 = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .css_classes(["start-screen-steps-box"])
            .build();

        let icon3: gtk::Image = get_image_from_path(
            modules.pages_settings.start_screen.real_time_icon.clone(),
            &["start-screen-steps-icon"],
        );

        let label3 = gtk::Label::builder()
            .label("Integrated metrics and logs collection, compatible with OpenTelemetry")
            .css_classes(["start-screen-steps-label"])
            .wrap(true)
            .wrap_mode(pango::WrapMode::Word)
            .vexpand(true)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::End)
            .justify(gtk::Justification::Center)
            .build();

        hbox_line3.append(&icon3);
        hbox_line3.append(&label3);

        let hbox_line4 = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .css_classes(["start-screen-steps-box"])
            .build();

        let icon4: gtk::Image = get_image_from_path(
            modules.pages_settings.start_screen.encypt_icon.clone(),
            &["start-screen-steps-icon"],
        );

        let label4: gtk::Label = gtk::Label::builder()
            .label("Identity management using secure x.509 certificates")
            .css_classes(["start-screen-steps-label"])
            .wrap(true)
            .wrap_mode(pango::WrapMode::Word)
            .vexpand(true)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::End)
            .justify(gtk::Justification::Center)
            .build();

        hbox_line4.append(&icon4);
        hbox_line4.append(&label4);

        let carousel = adw::Carousel::builder()
            .hexpand(true)
            .spacing(15)
            .width_request(340)
            .height_request(300)
            .css_classes(["carousel"])
            .build();

        carousel.append(&hbox_line2);
        carousel.append(&hbox_line3);
        carousel.append(&hbox_line4);

        let carousel_dots = adw::CarouselIndicatorDots::builder().build();
        carousel_dots.set_carousel(Some(&carousel));

        main_content_box.append(&carousel);
        main_content_box.append(&carousel_dots);

        let footer_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .valign(gtk::Align::End)
            .build();

        let back_icon_img: gtk::Image = get_image_from_path(widget_configs.footer.back_icon, &[]);
        let back_button_box = gtk::Box::builder().hexpand(true).build();
        let back_button = Button::builder().build();
        back_button.set_child(Some(&back_icon_img));
        back_button.add_css_class("footer-container-button");

        back_button.connect_clicked(clone!(@strong sender => move |_| {
          let _ =  sender.output(StartScreenOutput::BackPressed);
        }));

        let next_icon_img: gtk::Image = get_image_from_path(widget_configs.footer.next_icon, &[]);
        let next_button = Button::new();
        next_button.set_child(Some(&next_icon_img));
        next_button.add_css_class("footer-container-button");

        next_button.connect_clicked(clone!(@strong sender => move |_| {
            let sender: AsyncComponentSender<StartScreen> = sender.clone();
            let _ = check_machine_provision(sender);
        }));

        back_button_box.append(&back_button);
        footer_box.append(&back_button_box);
        footer_box.append(&next_button);

        footer_content_box.append(&footer_box);
        main_content_box.append(&footer_content_box);
        main_container.append(&main_content_box); // main box

        root.append(&main_container);

        let widgets = AppWidgets {};

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
    }
}

fn check_machine_provision(sender: AsyncComponentSender<StartScreen>) {
    let fn_name = "check_machine_provision";
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "Start screen - get provision status"
    );

    let _ = relm4::spawn(async move {
        let _ = match machine_provision_status().await {
            Ok(response) => {
                if response.status == true {
                    debug!(
                        fn_name,
                        package = PACKAGE_NAME,
                        "provision completed ==> check machine info"
                    );
                    let _ = sender
                        .output_sender()
                        .send(StartScreenOutput::ShowMachineInfo);
                } else {
                    debug!(
                        fn_name,
                        package = PACKAGE_NAME,
                        "provision not done ==> moving to check internet"
                    );
                    let _ = sender
                        .output_sender()
                        .send(StartScreenOutput::ShowCheckInternet);
                }
            }
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "Error - getting provision status{:?}",
                    e
                );
                let _ = sender
                    .output_sender()
                    .send(StartScreenOutput::ShowCheckInternet);
            }
        };
    });
}
