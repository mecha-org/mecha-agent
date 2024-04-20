pub mod errors;
mod handlers;
mod pages;
mod server;
mod services;
mod settings;

use async_trait::async_trait;
use gtk::prelude::{BoxExt, GtkWindowExt};
use handlers::machine_info;
use init_tracing_opentelemetry::tracing_subscriber_ext::{
    build_logger_text, build_loglevel_filter_layer, build_otel_layer,
};
use pages::{
    check_internet::{CheckInternet, CheckInternetOutput, Settings as CheckInternetSettings},
    configure_machine::{ConfigureMachine, ConfigureOutput, Settings as ConfigureMachineSettings},
    link_machine::{LinkMachine, LinkMachineOutput, Settings as LinkMachineSettings},
    machine_info::{DevicePageOutput, MachineInfo, Settings as DeviceInfoSettings},
    no_internet::{NoInternet, PageOutput, Settings as NoInternetSettings},
    setup_failed::{Settings as SetupFailedSettings, SetupFailOutput, SetupFailed},
    setup_success::{Settings as SetupSuccessSettings, SetupSuccess, SetupSuccessOutput},
    start_screen::{Settings as StartScreenSettings, StartScreen, StartScreenOutput},
    timeout_screen::{Settings as TimeoutScreenSettings, TimeoutOutput, TimeoutScreen},
};
use relm4::{
    component::{AsyncComponent, AsyncComponentController, AsyncComponentParts, AsyncController}, gtk::{glib::clone, prelude::{ApplicationExt, WidgetExt}}, AsyncComponentSender, RelmContainerExt, RelmSetChildExt, SimpleComponent
};
use relm4::{gtk, ComponentController};
use relm4::{Component, Controller, RelmApp};
use sentry_tracing::EventFilter;
use settings::{Modules, ScreenSettings, WidgetConfigs};
use std::{env, fmt};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::format, layer::SubscriberExt, EnvFilter};
mod utils;
mod widgets;
#[derive(Debug)]

struct ErrorMessage {
    error: String,
}

struct MechaConnectApp {
    current_screen: Pages,
    screen_stack: gtk::Stack,
    link_machine: AsyncController<LinkMachine>,
    start_screen: AsyncController<StartScreen>,
    check_internet: AsyncController<CheckInternet>,
    no_internet: Controller<NoInternet>,
    configure_machine: AsyncController<ConfigureMachine>,
    timeout_screen: Controller<TimeoutScreen>,
    setup_success: Controller<SetupSuccess>,
    setup_failed: Controller<SetupFailed>,
    machine_info: AsyncController<MachineInfo>,
    machine_id: String,
}

struct errorInfo {
    error_message: String,
    from_screen: String,
}

#[derive(Debug)]
enum Pages {
    StartScreen,
    CheckInternet,
    NoInternet,
    LinkMachine,
    ConfigureMachine,
    TimeoutScreen,
    SetupSuccess(String),
    SetupFailed(String, String),
    MachineInfo(String),
}

impl fmt::Display for Pages {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Pages::StartScreen => write!(f, "start_screen"),
            Pages::CheckInternet => write!(f, "check_internet"),
            Pages::NoInternet => write!(f, "no_internet"),
            Pages::LinkMachine => write!(f, "link_machine"),
            Pages::ConfigureMachine => write!(f, "configure_machine"),
            Pages::TimeoutScreen => write!(f, "timeout_screen"),
            Pages::SetupSuccess(machine_id) => write!(f, "setup_success"),
            Pages::SetupFailed(error, from_screen) => write!(f, "setup_failed"),
            Pages::MachineInfo(machine_id) => write!(f, "machine_info"),
        }
    }
}

#[derive(Debug)]
enum Message {
    ChangeScreen(Pages),
    Exit, // NextPressed
}
#[derive(Debug)]
enum AppInput {}

struct AppWidgets {
    screen_stack: gtk::Stack,
}

fn init_window(settings: ScreenSettings) -> gtk::Window {
    let window_settings = settings.window;
    let window = gtk::Window::builder()
        .title("Mecha Connect")
        // .default_width(window_settings.size.0)
        // .default_height(window_settings.size.1)
        .css_classes(["window"])
        .build();
    window.set_resizable(true);
    // window.set_default_size(window_settings.size.0, window_settings.size.1);
    println!("CHECK WINDOW SIZE {:?} ", window.default_size());
    window
}

#[async_trait(?Send)]
impl AsyncComponent for MechaConnectApp {
    type Input = Message;
    type Output = ();
    type Init = ();
    type Root = gtk::Window;
    type Widgets = AppWidgets;
    type CommandOutput = Message;

    fn init_root() -> Self::Root {
        let settings = match settings::read_settings_yml() {
            Ok(settings) => {
                println!("main::init_root--- {:?} ", settings);
                settings
            }
            Err(_) => ScreenSettings::default(),
        };

        tracing::info!(
            func = "initalize_settings",
            package = env!("CARGO_PKG_NAME"),
            "settings initialized for Lock Screen",
        );

        let window = init_window(settings);
        window
    }

    /// Initialize the UI and model.
    async fn init(
        _: Self::Init,
        window: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let settings = match settings::read_settings_yml() {
            Ok(settings) => {
                println!("main::init--- {:?} ", settings);
                settings
            }
            Err(_) => ScreenSettings::default(),
        };

        let start_screen = create_start_screen(
            settings.modules.clone(),
            settings.widget_configs.clone(),
            sender.input_sender().clone(),
        );

        let check_internet = create_check_internet_screen(
            settings.modules.clone(),
            settings.widget_configs.clone(),
            sender.input_sender().clone(),
        );

        let no_internet: Controller<NoInternet> = create_no_internet_screen(
            settings.modules.clone(),
            settings.widget_configs.clone(),
            sender.input_sender().clone(),
        );

        let link_machine = create_link_machine_screen(
            settings.modules.clone(),
            settings.widget_configs.clone(),
            sender.input_sender().clone(),
        );

        let configure_machine = create_configure_machine_screen(
            settings.modules.clone(),
            settings.widget_configs.clone(),
            sender.input_sender().clone(),
        );

        let timeout_screen = create_timeout_screen(
            settings.modules.clone(),
            settings.widget_configs.clone(),
            sender.input_sender().clone(),
        );

        let setup_success = create_setup_success_screen(
            settings.modules.clone(),
            settings.widget_configs.clone(),
            sender.input_sender().clone(),
        );

        let setup_failed = create_error_screen(
            settings.modules.clone(),
            settings.widget_configs.clone(),
            sender.input_sender().clone(),
        );

        let machine_info = create_machine_info_screen(
            settings.modules.clone(),
            settings.widget_configs.clone(),
            sender.input_sender().clone(),
        );

        let screen_stack = gtk::Stack::builder().build();

        screen_stack.add_named(
            start_screen.widget(),
            Option::from(Pages::StartScreen.to_string().as_str()),
        );

        screen_stack.add_named(
            check_internet.widget(),
            Option::from(Pages::CheckInternet.to_string().as_str()),
        );

        screen_stack.add_named(
            no_internet.widget(),
            Option::from(Pages::NoInternet.to_string().as_str()),
        );

        screen_stack.add_named(
            link_machine.widget(),
            Option::from(Pages::LinkMachine.to_string().as_str()),
        );

        screen_stack.add_named(
            configure_machine.widget(),
            Option::from(Pages::ConfigureMachine.to_string().as_str()),
        );

        screen_stack.add_named(
            timeout_screen.widget(),
            Option::from(Pages::TimeoutScreen.to_string().as_str()),
        );

        screen_stack.add_named(
            setup_success.widget(),
            Option::from(Pages::SetupSuccess(String::from("")).to_string().as_str()),
        );

        screen_stack.add_named(
            setup_failed.widget(),
            Option::from(
                Pages::SetupFailed(String::from(""), String::from(""))
                    .to_string()
                    .as_str(),
            ),
        );

        screen_stack.add_named(
            machine_info.widget(),
            Option::from(Pages::MachineInfo(String::from("")).to_string().as_str()),
        );

        //Setting current active screen in stack
        let current_screen = Pages::StartScreen; // OG
        screen_stack.set_visible_child_name(&current_screen.to_string());
        screen_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        screen_stack.set_transition_duration(300);
        
        // window.set_child(Some(&screen_stack));
        window.container_add(&screen_stack);

        let model = MechaConnectApp {
            current_screen,
            screen_stack: screen_stack.clone(),
            start_screen,
            check_internet,
            no_internet,
            link_machine,
            configure_machine,
            timeout_screen,
            setup_success,
            setup_failed,
            machine_info,
            machine_id: String::from("-"),
        };

        let widgets = AppWidgets { screen_stack };

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        let settings = match settings::read_settings_yml() {
            Ok(settings) => {
                println!("main::update--- {:?} ", settings);
                settings
            }
            Err(_) => ScreenSettings::default(),
        };

        match message {
            Message::ChangeScreen(page) => {
                __self.current_screen = page;

                match &self.current_screen {
                    Pages::StartScreen => {
                        // self.start_screen.detach_runtime();

                        let start_screen = create_start_screen(
                            settings.modules.clone(),
                            settings.widget_configs.clone(),
                            sender.input_sender().clone(),
                        );

                        self.screen_stack.remove(
                            &self
                                .screen_stack
                                .child_by_name(Pages::StartScreen.to_string().as_str())
                                .unwrap(),
                        );

                        self.screen_stack.add_named(
                            start_screen.widget(),
                            Option::from(Pages::StartScreen.to_string().as_str()),
                        );

                        self.start_screen = start_screen;
                    }
                    Pages::CheckInternet => {
                        self.check_internet.detach_runtime();

                        let check_internet = create_check_internet_screen(
                            settings.modules.clone(),
                            settings.widget_configs.clone(),
                            sender.input_sender().clone(),
                        );

                        self.screen_stack.remove(
                            &self
                                .screen_stack
                                .child_by_name(Pages::CheckInternet.to_string().as_str())
                                .unwrap(),
                        );

                        self.screen_stack.add_named(
                            check_internet.widget(),
                            Option::from(Pages::CheckInternet.to_string().as_str()),
                        );

                        self.check_internet = check_internet;

                        let _ = self.check_internet.sender().send(
                            pages::check_internet::InputMessage::ActiveScreen(
                                self.current_screen.to_string(),
                            ),
                        );
                    }
                    Pages::NoInternet => {
                        self.no_internet.detach_runtime();

                        let no_internet = create_no_internet_screen(
                            settings.modules.clone(),
                            settings.widget_configs.clone(),
                            sender.input_sender().clone(),
                        );

                        self.screen_stack.remove(
                            &self
                                .screen_stack
                                .child_by_name(Pages::NoInternet.to_string().as_str())
                                .unwrap(),
                        );

                        self.screen_stack.add_named(
                            no_internet.widget(),
                            Option::from(Pages::NoInternet.to_string().as_str()),
                        );

                        self.no_internet = no_internet;
                    }
                    Pages::LinkMachine => {
                        self.link_machine.detach_runtime();
                        let link_machine = create_link_machine_screen(
                            settings.modules.clone(),
                            settings.widget_configs.clone(),
                            sender.input_sender().clone(),
                        );

                        self.screen_stack.remove(
                            &self
                                .screen_stack
                                .child_by_name(Pages::LinkMachine.to_string().as_str())
                                .unwrap(),
                        );

                        self.screen_stack.add_named(
                            link_machine.widget(),
                            Option::from(Pages::LinkMachine.to_string().as_str()),
                        );

                        self.link_machine = link_machine;

                        let _ = self.link_machine.sender().send(
                            pages::link_machine::InputMessage::ActiveScreen(
                                self.current_screen.to_string(),
                            ),
                        );
                    }
                    Pages::ConfigureMachine => {
                        let configure_machine = create_configure_machine_screen(
                            settings.modules.clone(),
                            settings.widget_configs.clone(),
                            sender.input_sender().clone(),
                        );

                        self.screen_stack.remove(
                            &self
                                .screen_stack
                                .child_by_name(Pages::ConfigureMachine.to_string().as_str())
                                .unwrap(),
                        );

                        self.screen_stack.add_named(
                            configure_machine.widget(),
                            Option::from(Pages::ConfigureMachine.to_string().as_str()),
                        );

                        self.configure_machine = configure_machine;

                        let _ = __self.configure_machine.sender().send(
                            pages::configure_machine::InputMessage::ActiveScreen(
                                self.current_screen.to_string(),
                            ),
                        );
                    }
                    Pages::TimeoutScreen => {
                        let timeout_screen = create_timeout_screen(
                            settings.modules.clone(),
                            settings.widget_configs.clone(),
                            sender.input_sender().clone(),
                        );
                        self.screen_stack.remove(
                            &self
                                .screen_stack
                                .child_by_name(Pages::TimeoutScreen.to_string().as_str())
                                .unwrap(),
                        );
                        self.screen_stack.add_named(
                            timeout_screen.widget(),
                            Option::from(Pages::TimeoutScreen.to_string().as_str()),
                        );

                        self.timeout_screen = timeout_screen;
                    }
                    Pages::SetupSuccess(machine_id) => {
                        println!("RENDER::SetupSuccess machine_id {:?}", machine_id.clone());

                        self.setup_success.detach_runtime();

                        self.machine_id = machine_id.clone();

                        let setup_success = create_setup_success_screen(
                            settings.modules.clone(),
                            settings.widget_configs.clone(),
                            sender.input_sender().clone(),
                        );

                        self.screen_stack.remove(
                            &self
                                .screen_stack
                                .child_by_name(
                                    Pages::SetupSuccess(String::from("")).to_string().as_str(),
                                )
                                .unwrap(),
                        );

                        self.screen_stack.add_named(
                            setup_success.widget(),
                            Option::from(
                                Pages::SetupSuccess(String::from("")).to_string().as_str(),
                            ),
                        );

                        self.setup_success = setup_success;
                        let _ = __self.setup_success.sender().send(
                            pages::setup_success::InputMessage::ActiveScreen(
                                self.current_screen.to_string(),
                            ),
                        );
                    }
                    Pages::SetupFailed(error, from_screen) => {
                        self.setup_failed.detach_runtime();

                        let setup_failed = create_error_screen(
                            settings.modules.clone(),
                            settings.widget_configs.clone(),
                            sender.input_sender().clone(),
                        );

                        self.screen_stack.remove(
                            &self
                                .screen_stack
                                .child_by_name(
                                    Pages::SetupFailed(String::from(""), String::from(""))
                                        .to_string()
                                        .as_str(),
                                )
                                .unwrap(),
                        );

                        self.screen_stack.add_named(
                            setup_failed.widget(),
                            Option::from(
                                Pages::SetupFailed(String::from(""), String::from(""))
                                    .to_string()
                                    .as_str(),
                            ),
                        );

                        self.setup_failed = setup_failed;

                        let _ = self.setup_failed.sender().send(
                            pages::setup_failed::InputMessage::ShowError(
                                error.to_string(),
                                from_screen.to_string(),
                            ),
                        );
                    }
                    Pages::MachineInfo(machine_id_value) => {
                        println!(
                            "RENDER:: MachineInfo :: machine_id_value {:?}",
                            machine_id_value
                        );
                        let machine_info_screen = create_machine_info_screen(
                            settings.modules.clone(),
                            settings.widget_configs.clone(),
                            sender.input_sender().clone(),
                        );
                        self.screen_stack.remove(
                            &self
                                .screen_stack
                                .child_by_name(
                                    Pages::MachineInfo(String::from("")).to_string().as_str(),
                                )
                                .unwrap(),
                        );
                        self.screen_stack.add_named(
                            machine_info_screen.widget(),
                            Option::from(
                                Pages::MachineInfo(machine_id_value.to_owned())
                                    .to_string()
                                    .as_str(),
                            ),
                        );

                        self.machine_info = machine_info_screen;

                        let mut machine_id = self.machine_id.clone();
                        if machine_id == "-" || machine_id == "-" {
                            machine_id = machine_id_value.to_owned();
                        }

                        let _ = __self
                            .machine_info
                            .sender()
                            .send(pages::machine_info::InputMessage::ActiveScreen(machine_id));

                        let _ = __self
                        .machine_info
                        .sender()
                        .send(pages::machine_info::InputMessage::GetInformation);
                    }
                }
            }
            Message::Exit => relm4::main_application().quit(),
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: AsyncComponentSender<Self>) {
        widgets
            .screen_stack
            .set_visible_child_name(self.current_screen.to_string().as_str());
    }
}

fn create_start_screen(
    modules: Modules,
    widget_configs: WidgetConfigs,
    sender: relm4::Sender<Message>,
) -> AsyncController<StartScreen> {
    let start_screen: AsyncController<StartScreen> = StartScreen::builder()
        .launch(StartScreenSettings {
            modules: modules.clone(),
            widget_configs: widget_configs.clone(),
        })
        .forward(
            &sender,
            clone!(@strong modules => move|msg| match msg {
                StartScreenOutput::BackPressed => Message::Exit,
                StartScreenOutput::ShowMachineInfo(machine_id) => Message::ChangeScreen(Pages::MachineInfo(machine_id)),
                StartScreenOutput::ShowCheckInternet => Message::ChangeScreen(Pages::CheckInternet),        // OG
                
                // StartScreenOutput::ShowMachineInfo(machine_id) => Message::ChangeScreen(Pages::ConfigureMachine), // TEMP - TO TEST
            }),
        );

    start_screen
}

fn create_check_internet_screen(
    modules: Modules,
    widget_configs: WidgetConfigs,
    sender: relm4::Sender<Message>,
) -> AsyncController<CheckInternet> {
    let check_internet = CheckInternet::builder()
        .launch(CheckInternetSettings {
            modules: modules.clone(),
            widget_configs: widget_configs.clone(),
        })
        .forward(&sender, move |msg| match msg {
            CheckInternetOutput::BackPressed => Message::ChangeScreen(Pages::StartScreen),
            CheckInternetOutput::LinkMachine => Message::ChangeScreen(Pages::LinkMachine),
            CheckInternetOutput::ConnectionNotFound => Message::ChangeScreen(Pages::NoInternet),
            CheckInternetOutput::ShowError(error) => {
                Message::ChangeScreen(Pages::SetupFailed(error, "check_internet".to_string()))
            }
        });
    check_internet
}

fn create_no_internet_screen(
    modules: Modules,
    widget_configs: WidgetConfigs,
    sender: relm4::Sender<Message>,
) -> Controller<NoInternet> {
    let no_internet = NoInternet::builder()
        .launch(NoInternetSettings {
            modules: modules.clone(),
            widget_configs: widget_configs.clone(),
        })
        .forward(
            &sender,
            clone!(@strong modules => move|msg| match msg {
                PageOutput::BackPressed => Message::ChangeScreen(Pages::CheckInternet),
                // TODO : navigate
                PageOutput::SettingsPressed => Message::ChangeScreen(Pages::CheckInternet)
            }),
        );
    no_internet
}

fn create_link_machine_screen(
    modules: Modules,
    widget_configs: WidgetConfigs,
    sender: relm4::Sender<Message>,
) -> AsyncController<LinkMachine> {
    let link_machine = LinkMachine::builder().launch(LinkMachineSettings{
            modules: modules.clone(),
            widget_configs: widget_configs.clone()
        })
        .forward(
            &sender,
            clone!(@strong modules => move|msg| match msg {
                LinkMachineOutput::BackPressed => Message::ChangeScreen(Pages::CheckInternet),
                LinkMachineOutput::NextPressed => Message::ChangeScreen(Pages::ConfigureMachine),
                LinkMachineOutput::ShowError(error) => Message::ChangeScreen(Pages::SetupFailed(error, "link_machine".to_owned())),
            }),
        );
    link_machine
}

fn create_configure_machine_screen(
    modules: Modules,
    widget_configs: WidgetConfigs,
    sender: relm4::Sender<Message>,
) -> AsyncController<ConfigureMachine> {
    let configure_machine = ConfigureMachine::builder()
    .launch(ConfigureMachineSettings {
        modules: modules.clone(),
        widget_configs: widget_configs.clone(),
    })
    .forward(
        &sender,
        clone!(@strong modules => move|msg| match msg {
            ConfigureOutput::Timeout => Message::ChangeScreen(Pages::TimeoutScreen),
            ConfigureOutput::SetupSuccess(machine_id) =>  Message::ChangeScreen(Pages::SetupSuccess(machine_id)),
            ConfigureOutput::ShowError(error) => {
                Message::ChangeScreen(Pages::SetupFailed(error, "configure_machine".to_string()))
            },

        }),
    );
    configure_machine
}

fn create_timeout_screen(
    modules: Modules,
    widget_configs: WidgetConfigs,
    sender: relm4::Sender<Message>,
) -> Controller<TimeoutScreen> {
    let timeout_screen = TimeoutScreen::builder()
        .launch(TimeoutScreenSettings {
            modules: modules.clone(),
            widget_configs: widget_configs.clone(),
        })
        .forward(
            &sender,
            clone!(@strong modules => move|msg| match msg {
                TimeoutOutput::refreshPressed => Message::ChangeScreen(Pages::ConfigureMachine),
                TimeoutOutput::BackPressed => Message::ChangeScreen(Pages::ConfigureMachine)
            }),
        );
    timeout_screen
}

fn create_setup_success_screen(
    modules: Modules,
    widget_configs: WidgetConfigs,
    sender: relm4::Sender<Message>,
) -> Controller<SetupSuccess> {
    let setup_success = SetupSuccess::builder()
        .launch(SetupSuccessSettings {
            modules: modules.clone(),
            widget_configs: widget_configs.clone(),
        })
        .forward(
            &sender,
            clone!(@strong modules => move|msg| match msg {
                SetupSuccessOutput::BackPressed => Message::ChangeScreen(Pages::ConfigureMachine),  // remove
                SetupSuccessOutput::NextPressed =>
                Message::ChangeScreen(Pages::MachineInfo(String::from("")))
            }),
        );
    setup_success
}

fn create_error_screen(
    modules: Modules,
    widget_configs: WidgetConfigs,
    sender: relm4::Sender<Message>,
) -> Controller<SetupFailed> {
    let setup_failed: Controller<SetupFailed> = SetupFailed::builder()
    .launch(SetupFailedSettings {
        modules: modules.clone(),
        widget_configs: widget_configs.clone(),
    })
    .forward(
        &sender,
        clone!(@strong modules => move|msg| match msg {
            SetupFailOutput::refresh(screen) => {
                info!("REFRESH SCREEN: {:?}", screen.to_owned());

                match screen {
                    screen if screen == String::from("check_internet") =>  Message::ChangeScreen(Pages::CheckInternet),
                    screen if screen == String::from("configure_machine") =>  Message::ChangeScreen(Pages::ConfigureMachine),
                    screen if screen == String::from("link_machine") =>  Message::ChangeScreen(Pages::LinkMachine),
                    _ => {
                        println!("Found something else");
                    Message::ChangeScreen(Pages::StartScreen)},
                }
            }

        }),
    );

    setup_failed
}

fn create_machine_info_screen(
    modules: Modules,
    widget_configs: WidgetConfigs,
    sender: relm4::Sender<Message>,
) -> AsyncController<MachineInfo> {
    let machine_info = MachineInfo::builder()
        .launch(DeviceInfoSettings {
            modules: modules.clone(),
            widget_configs: widget_configs.clone(),
        })
        .forward(
            &sender,
            clone!(@strong modules => move|msg| match msg {
                DevicePageOutput::Exit=>Message::Exit,
            }),
        );
    machine_info
}

#[tokio::main]
async fn main() {
    let app = RelmApp::new("mecha.connect.app");

    let settings = match settings::read_settings_yml() {
        Ok(settings) => {
            println!("main::main--- {:?} ", settings);
            settings
        }
        Err(_) => ScreenSettings::default(),
    };

    let css = settings.css.clone();
    let _ = app.set_global_css_from_file(css.default);

    env::set_var("RUST_LOG", "connect=trace,debug,info,error,warn");

    let subscriber = tracing_subscriber::registry()
        .with(sentry_tracing::layer().event_filter(|_| EventFilter::Ignore))
        .with(build_loglevel_filter_layer())
        .with(build_logger_text())
        // .with(
        //     EnvFilter::try_new("connect")
        //         .unwrap_or_else(|_| EnvFilter::new::<String>(LevelFilter::current().to_string())),
        // )
        .with(build_otel_layer().unwrap());

    match tracing::subscriber::set_global_default(subscriber) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Error setting global default subscriber: {}", e)
        }
    };

    tracing::info!(
        func = "set_tracing",
        package = env!("CARGO_PKG_NAME"),
        "tracing set up - info",
    );

    app.run_async::<MechaConnectApp>(());
}
