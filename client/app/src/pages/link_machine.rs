use crate::{
    handlers::provision::handler::LinkMachineHandler,
    settings::{Modules, WidgetConfigs},
    utils,
};
use async_trait::async_trait;
use gtk::prelude::*;
use relm4::{
    component::{AsyncComponent, AsyncComponentParts},
    gtk::{
        self,
        gdk::Display,
        glib::{self, clone},
        pango,
        prelude::{ButtonExt, WidgetExt},
        CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION,
    },
    AsyncComponentSender,
};
use std::time::Duration;
use tracing::info;
use utils::get_image_from_path;
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

pub struct Settings {
    pub modules: Modules,
    pub widget_configs: WidgetConfigs,
}

pub struct LinkMachine {
    settings: Settings,
    connect_code: String,
    progress: f64,
    timer: i32,
    provision_status: bool,
    current_time: i32,
    task: Option<glib::JoinHandle<()>>,
    g: Option<tokio::task::JoinHandle<()>>,
    p: Option<tokio::task::JoinHandle<()>>,
    t: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug)]
pub enum LinkMachineOutput {
    BackPressed,
    NextPressed,
    ShowError(String),
}

#[derive(Debug)]
pub enum InputMessage {
    ActiveScreen(String),
    CodeChanged(String),
    UpdateTimer(f64),
    GenerateCodeError(String),
    ProvisionSuccess,
    ShowError(String),
    BackScreen,
    ProvisioningTasks {
        g: tokio::task::JoinHandle<()>,
        p: tokio::task::JoinHandle<()>,
        // t: tokio::task::JoinHandle<()>,
    },
}

pub struct AppWidgets {
    connect_code_label: gtk::Label,
    progress_bar: gtk::ProgressBar,
    // spinner: gtk::Spinner,
    // timer_label: gtk::Label,
}

const TIMER: i32 = 10;

#[async_trait(?Send)]
impl AsyncComponent for LinkMachine {
    type Init = Settings;
    type Input = InputMessage;
    type Output = LinkMachineOutput;
    type Root = gtk::Box;
    type Widgets = AppWidgets;
    type CommandOutput = ();

    fn init_root() -> Self::Root {
        let provider = CssProvider::new();
        //provider.load_from_data(include_str!("../assets/css/style.css"));
        gtk::style_context_add_provider_for_display(
            &Display::default().expect("Could not connect to a display."),
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        gtk::Box::builder().build()
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let modules = init.modules.clone();
        let widget_configs = init.widget_configs.clone();

        let main_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let main_content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(["app-container"])
            .build();

        let footer_content_box: gtk::Box = gtk::Box::builder()
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
            .label("Link Your Machine")
            .halign(gtk::Align::Start)
            .build();

        header_label
            .style_context()
            .add_class("start-screen-header");

        header_box.append(&app_icon);
        header_box.append(&header_label);

        main_container.append(&header_box); // main box

        let header_info_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            // .css_classes(["start-header-p"])
            .build();

        let info_sentence = gtk::Label::builder()
            .label("Use this below code to connect this machine to your Mecha account")
            .halign(gtk::Align::Start)
            .build();

        header_info_box.append(&info_sentence);
        main_content_box.append(&header_info_box);

        let info_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(["start-screen-steps-container"])
            .build();

        // check-code
        let main_code_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .css_classes(["link-machine-border-box"])
            .build();

        let code_label_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build();

        // spinner
        let spinner = gtk::Spinner::builder()
            .css_classes(["blue"])
            .height_request(30)
            .width_request(30)
            .build();
        spinner.set_spinning(false);

        let connect_code_label = gtk::Label::builder()
            .label("") // ABCD 1234
            .css_classes(["link-machine-code"])
            .build();

        // progress bar
        let progress_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            // .css_classes(["link-machine-progress-box"])
            .build();

        // progressbar
        let progress_bar = gtk::ProgressBar::builder()
            .fraction(1.0)
            .hexpand(true)
            .build();
        progress_bar
            .style_context()
            .add_class("custom-progress-bar");

        progress_box.append(&progress_bar);

        code_label_box.append(&connect_code_label);
        main_code_box.append(&code_label_box);
        // main_code_box.append(&spinner);

        info_box.append(&main_code_box);
        info_box.append(&progress_box);
        // main_content_box.append(&main_code_box);

        let main_steps_box: gtk::Box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(["link-machine-steps-container"])
            .build();

        let linking_step1_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .css_classes(["link-machine-steps-box"])
            .hexpand(true)
            .build();

        let step1_label_box = gtk::Box::builder()
            .css_classes(["circle-border-box"])
            .valign(gtk::Align::Start)
            .build();

        let step1_label = gtk::Label::builder()
            .label("1")
            .width_request(25)
            .height_request(25)
            .build();
        step1_label_box.append(&step1_label);

        let step1_text = gtk::Label::builder()
            .label("Create a new account on Mecha, if not signed up earlier.")
            .css_classes(["link-machine-steps-text"])
            .wrap(true)
            .wrap_mode(pango::WrapMode::Word)
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build();

        linking_step1_box.append(&step1_label_box);
        linking_step1_box.append(&step1_text);

        main_steps_box.append(&linking_step1_box);

        //
        let linking_step2_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .css_classes(["link-machine-steps-box"])
            .hexpand(true)
            .build();

        let step2_label_box = gtk::Box::builder()
            .css_classes(["circle-border-box"])
            .valign(gtk::Align::Start)
            .build();

        let step2_label = gtk::Label::builder()
            .label("2")
            .width_request(25)
            .height_request(25)
            .build();
        step2_label_box.append(&step2_label);

        let step2_text = gtk::Label::builder()
            .label("Navigate to Machines > Add Machine")
            .css_classes(["link-machine-steps-text"])
            .build();

        linking_step2_box.append(&step2_label_box);
        linking_step2_box.append(&step2_text);

        main_steps_box.append(&linking_step2_box);

        //
        let linking_step3_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .css_classes(["link-machine-steps-box"])
            .hexpand(true)
            .build();

        let step3_label_box = gtk::Box::builder()
            .css_classes(["circle-border-box"])
            .valign(gtk::Align::Start)
            .build();

        let step3_label = gtk::Label::builder()
            .label("3")
            .width_request(25)
            .height_request(25)
            .build();
        step3_label_box.append(&step3_label);

        let step3_text = gtk::Label::builder()
            .label("Enter the code shown above when asked")
            .css_classes(["link-machine-steps-text"])
            .build();

        linking_step3_box.append(&step3_label_box);
        linking_step3_box.append(&step3_text);

        main_steps_box.append(&linking_step3_box);

        info_box.append(&main_steps_box);
        main_content_box.append(&info_box);

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
            let _ =  sender.input_sender().send(InputMessage::BackScreen);
        }));

        back_button_box.append(&back_button);
        footer_box.append(&back_button_box);

        footer_content_box.append(&footer_box);
        main_content_box.append(&footer_content_box);
        main_container.append(&main_content_box);

        root.append(&main_container);

        let model = LinkMachine {
            settings: init,
            connect_code: "".to_string(),
            timer: TIMER,
            provision_status: false,
            progress: 0.0,
            current_time: 0,
            task: None,
            g: None,
            p: None,
            t: None,
        };

        let widgets = AppWidgets {
            connect_code_label,
            progress_bar,
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
            InputMessage::ActiveScreen(text) => {
                info!("active screen: {:?}", text);
                let sender: relm4::Sender<InputMessage> = sender.input_sender().clone();
                let relm_task: glib::JoinHandle<()> = relm4::spawn_local(async move {
                    let _ = link_machine_init_services(sender).await;
                });
                self.task = Some(relm_task)
            }
            InputMessage::ProvisionSuccess => {
                self.task.as_ref().unwrap().abort();
                // self.t.as_ref().unwrap().abort();
                let _ = sender.output(LinkMachineOutput::NextPressed);
            }
            InputMessage::CodeChanged(code) => {
                self.connect_code = code.clone();
            }
            InputMessage::UpdateTimer(time_fraction) => {
                self.progress = time_fraction.clone();
            }
            InputMessage::GenerateCodeError(error) => {
                self.task.as_ref().unwrap().abort();
                self.g.as_ref().unwrap().abort();
                self.p.as_ref().unwrap().abort();

                let _ = tokio::time::sleep(Duration::from_millis(5000)).await;
                let _ = sender.output(LinkMachineOutput::ShowError(error));
            }
            InputMessage::ShowError(error) => {
                self.task.as_ref().unwrap().abort();
                self.g.as_ref().unwrap().abort();
                self.p.as_ref().unwrap().abort();
                let _ = sender.output(LinkMachineOutput::ShowError(error));
            }
            InputMessage::BackScreen => {
                self.task.as_ref().unwrap().abort();
                self.g.as_ref().unwrap().abort();
                self.p.as_ref().unwrap().abort();
                // self.t.as_ref().unwrap().abort();

                let _ = sender.output(LinkMachineOutput::BackPressed);
            }
            InputMessage::ProvisioningTasks { g, p } => {
                self.g = Some(g);
                self.p = Some(p);
                // self.t = Some(t);
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: AsyncComponentSender<Self>) {
        widgets.connect_code_label.set_label(&self.connect_code);
        widgets.progress_bar.set_fraction(self.progress);
    }
}

async fn link_machine_init_services(sender: relm4::Sender<InputMessage>) {
    let fn_name = "link_machine_init_services";
    info!(func = fn_name, package = PACKAGE_NAME);

    let sender_clone_1 = sender.clone();
    let mut link_machine_handler = LinkMachineHandler::new();

    let _ = relm4::spawn_local(async move {
        let _ = link_machine_handler.run(sender_clone_1).await;
    });
}
