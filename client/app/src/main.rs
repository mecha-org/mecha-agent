pub mod errors;
mod handlers;
mod pages;
mod server;
mod services;
mod settings;

use async_trait::async_trait;
use gtk::prelude::{BoxExt, GtkWindowExt};
use pages::{
    check_internet::{self, CheckInternet, CheckInternetOutput, Settings as CheckInternetSettings},
    configure_machine::{ConfigureMachine, ConfigureOutput, Settings as ConfigureMachineSettings},
    link_machine::{self, LinkMachine, LinkMachineOutput, Settings as LinkMachineSettings},
    machine_info::{DevicePageOutput, MachineInfo, Settings as DeviceInfoSettings},
    no_internet::{NoInternet, PageOutput, Settings as NoInternetSettings},
    setup_failed::{self, Settings as SetupFailedSettings, SetupFailOutput, SetupFailed},
    setup_success::{Settings as SetupSuccessSettings, SetupSuccess, SetupSuccessOutput},
    start_screen::{Settings as StartScreenSettings, StartScreen, StartScreenOutput},
    timeout_screen::{Settings as TimeoutScreenSettings, TimeoutOutput, TimeoutScreen},
};
use relm4::{
    component::{AsyncComponent, AsyncComponentController, AsyncComponentParts, AsyncController},
    gtk::{glib::clone, prelude::ApplicationExt},
    AsyncComponentSender, SimpleComponent,
};
use relm4::{gtk, ComponentController};
use relm4::{Component, Controller, RelmApp};
use services::MachineInformation;
use settings::{Modules, ScreenSettings, WidgetConfigs};
use std::fmt;
use tracing::info;

#[derive(Debug)]

struct ErrorMessage {
    error: String,
}

struct MechaConnectApp {
    current_page: Pages,
    pages_stack: gtk::Stack,
    link_machine: AsyncController<LinkMachine>,
    start_screen: Controller<StartScreen>,
    check_internet: AsyncController<CheckInternet>,
    no_internet: Controller<NoInternet>,
    configure_machine: AsyncController<ConfigureMachine>,
    timeout_screen: Controller<TimeoutScreen>,
    setup_success: Controller<SetupSuccess>,
    setup_failed: Controller<SetupFailed>,
    machine_info: AsyncController<MachineInfo>,
    machine_info_data: Option<MachineInformation>,
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
    SetupSuccess(MachineInformation),
    SetupFailed(String, String),
    MachineInfo,
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
            Pages::SetupSuccess(machine_info) => write!(f, "setup_success"),
            Pages::SetupFailed(error, from_screen) => write!(f, "setup_failed"),
            Pages::MachineInfo => write!(f, "machine_info"),
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
    pages_stack: gtk::Stack,
}

fn init_window(settings: ScreenSettings) -> gtk::Window {
    let window_settings = settings.window;
    let window = gtk::Window::builder()
        .title("Mecha Connect")
        .default_width(window_settings.size.0)
        .default_height(window_settings.size.1)
        .css_classes(["window"])
        .build();
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
            Ok(settings) => settings,
            Err(_) => ScreenSettings::default(),
        };

        info!(
            task = "initalize_settings",
            "settings initialized for Lock Screen: {:?}", settings
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
            Ok(settings) => settings,
            Err(_) => ScreenSettings::default(),
        };

        let css = settings.css.clone();
        // relm4::set_global_css_from_file(css.default);

        let modules = settings.modules.clone();
        let widget_configs = settings.widget_configs.clone();

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

        let pages_stack = gtk::Stack::builder().build();

        pages_stack.add_named(
            start_screen.widget(),
            Option::from(Pages::StartScreen.to_string().as_str()),
        );

        pages_stack.add_named(
            check_internet.widget(),
            Option::from(Pages::CheckInternet.to_string().as_str()),
        );

        pages_stack.add_named(
            no_internet.widget(),
            Option::from(Pages::NoInternet.to_string().as_str()),
        );

        pages_stack.add_named(
            link_machine.widget(),
            Option::from(Pages::LinkMachine.to_string().as_str()),
        );

        pages_stack.add_named(
            configure_machine.widget(),
            Option::from(Pages::ConfigureMachine.to_string().as_str()),
        );

        pages_stack.add_named(
            timeout_screen.widget(),
            Option::from(Pages::TimeoutScreen.to_string().as_str()),
        );

        pages_stack.add_named(
            setup_success.widget(),
            Option::from(
                Pages::SetupSuccess(MachineInformation::new())
                    .to_string()
                    .as_str(),
            ),
        );

        pages_stack.add_named(
            setup_failed.widget(),
            Option::from(
                Pages::SetupFailed("".to_owned(), "".to_owned())
                    .to_string()
                    .as_str(),
            ),
        );

        pages_stack.add_named(
            machine_info.widget(),
            Option::from(Pages::MachineInfo.to_string().as_str()),
        );

        let current_page = Pages::StartScreen; // OG

        //Setting current active screen in stack
        pages_stack.set_visible_child_name(&current_page.to_string());

        // add pages here
        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(5)
            .hexpand(true)
            .build();

        vbox.append(&pages_stack);

        let model = MechaConnectApp {
            current_page,
            pages_stack: pages_stack.clone(),
            start_screen,
            check_internet,
            no_internet,
            link_machine,
            configure_machine,
            timeout_screen,
            setup_success,
            setup_failed,
            machine_info,
            machine_info_data: None,
        };

        window.set_child(Some(&vbox));
        let widgets = AppWidgets { pages_stack };

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        let settings = match settings::read_settings_yml() {
            Ok(settings) => settings,
            Err(_) => ScreenSettings::default(),
        };

        match message {
            Message::ChangeScreen(page) => {
                __self.current_page = page;

                match &self.current_page {
                    Pages::StartScreen => {
                        // self.start_screen.detach_runtime();

                        let start_screen = create_start_screen(
                            settings.modules.clone(),
                            settings.widget_configs.clone(),
                            sender.input_sender().clone(),
                        );

                        self.pages_stack.remove(
                            &self
                                .pages_stack
                                .child_by_name(Pages::StartScreen.to_string().as_str())
                                .unwrap(),
                        );

                        self.pages_stack.add_named(
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

                        self.pages_stack.remove(
                            &self
                                .pages_stack
                                .child_by_name(Pages::CheckInternet.to_string().as_str())
                                .unwrap(),
                        );

                        self.pages_stack.add_named(
                            check_internet.widget(),
                            Option::from(Pages::CheckInternet.to_string().as_str()),
                        );

                        self.check_internet = check_internet;

                        let _ = self.check_internet.sender().send(
                            pages::check_internet::InputMessage::ActiveScreen(
                                self.current_page.to_string(),
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

                        self.pages_stack.remove(
                            &self
                                .pages_stack
                                .child_by_name(Pages::NoInternet.to_string().as_str())
                                .unwrap(),
                        );

                        self.pages_stack.add_named(
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

                        self.pages_stack.remove(
                            &self
                                .pages_stack
                                .child_by_name(Pages::LinkMachine.to_string().as_str())
                                .unwrap(),
                        );

                        self.pages_stack.add_named(
                            link_machine.widget(),
                            Option::from(Pages::LinkMachine.to_string().as_str()),
                        );

                        self.link_machine = link_machine;

                        let _ = self.link_machine.sender().send(
                            pages::link_machine::InputMessage::ActiveScreen(
                                self.current_page.to_string(),
                            ),
                        );
                    }
                    Pages::ConfigureMachine => {
                        let configure_machine = create_configure_machine_screen(
                            settings.modules.clone(),
                            settings.widget_configs.clone(),
                            sender.input_sender().clone(),
                        );

                        self.pages_stack.remove(
                            &self
                                .pages_stack
                                .child_by_name(Pages::ConfigureMachine.to_string().as_str())
                                .unwrap(),
                        );

                        self.pages_stack.add_named(
                            configure_machine.widget(),
                            Option::from(Pages::ConfigureMachine.to_string().as_str()),
                        );

                        self.configure_machine = configure_machine;

                        let _ = __self.configure_machine.sender().send(
                            pages::configure_machine::InputMessage::ActiveScreen(
                                self.current_page.to_string(),
                            ),
                        );
                    }
                    Pages::TimeoutScreen => {
                        let timeout_screen = create_timeout_screen(
                            settings.modules.clone(),
                            settings.widget_configs.clone(),
                            sender.input_sender().clone(),
                        );
                        self.pages_stack.remove(
                            &self
                                .pages_stack
                                .child_by_name(Pages::TimeoutScreen.to_string().as_str())
                                .unwrap(),
                        );
                        self.pages_stack.add_named(
                            timeout_screen.widget(),
                            Option::from(Pages::TimeoutScreen.to_string().as_str()),
                        );

                        self.timeout_screen = timeout_screen;
                    }
                    Pages::SetupSuccess(machine_info) => {
                        self.machine_info_data = Some(machine_info.clone());

                        self.setup_success.detach_runtime();

                        let setup_success = create_setup_success_screen(
                            settings.modules.clone(),
                            settings.widget_configs.clone(),
                            sender.input_sender().clone(),
                        );

                        self.pages_stack.remove(
                            &self
                                .pages_stack
                                .child_by_name(
                                    Pages::SetupSuccess(machine_info.clone())
                                        .to_string()
                                        .as_str(),
                                )
                                .unwrap(),
                        );

                        self.pages_stack.add_named(
                            setup_success.widget(),
                            Option::from(
                                Pages::SetupSuccess(machine_info.clone())
                                    .to_string()
                                    .as_str(),
                            ),
                        );

                        self.setup_success = setup_success;
                        let _ = __self.setup_success.sender().send(
                            pages::setup_success::InputMessage::ActiveScreen(
                                self.current_page.to_string(),
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

                        self.pages_stack.remove(
                            &self
                                .pages_stack
                                .child_by_name(
                                    Pages::SetupFailed("".to_owned(), "".to_owned())
                                        .to_string()
                                        .as_str(),
                                )
                                .unwrap(),
                        );

                        self.pages_stack.add_named(
                            setup_failed.widget(),
                            Option::from(
                                Pages::SetupFailed("".to_owned(), "".to_owned())
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
                    Pages::MachineInfo => {
                        let machine_info_screen = create_machine_info_screen(
                            settings.modules.clone(),
                            settings.widget_configs.clone(),
                            sender.input_sender().clone(),
                        );
                        self.pages_stack.remove(
                            &self
                                .pages_stack
                                .child_by_name(Pages::MachineInfo.to_string().as_str())
                                .unwrap(),
                        );
                        self.pages_stack.add_named(
                            machine_info_screen.widget(),
                            Option::from(Pages::MachineInfo.to_string().as_str()),
                        );

                        self.machine_info = machine_info_screen;

                        let data = self.machine_info_data.clone();
                        let _ = __self
                            .machine_info
                            .sender()
                            .send(pages::machine_info::InputMessage::ActiveScreen(data));
                    }
                }
            }
            Message::Exit => relm4::main_application().quit(),
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: AsyncComponentSender<Self>) {
        widgets
            .pages_stack
            .set_visible_child_name(self.current_page.to_string().as_str());
    }
}

fn create_start_screen(
    modules: Modules,
    widget_configs: WidgetConfigs,
    sender: relm4::Sender<Message>,
) -> Controller<StartScreen> {
    let start_screen: Controller<StartScreen> = StartScreen::builder()
        .launch(StartScreenSettings {
            modules: modules.clone(),
            widget_configs: widget_configs.clone(),
        })
        .forward(
            &sender,
            clone!(@strong modules => move|msg| match msg {
                StartScreenOutput::BackPressed => Message::ChangeScreen(Pages::StartScreen),
                StartScreenOutput::NextPressed => Message::ChangeScreen(Pages::CheckInternet)        // OG
                // StartScreenOutput::NextPressed => Message::ChangeScreen(Pages::ConfigureMachine)        // TEMP - TO TEST
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
        .forward(&sender, move |msg| {
            info!("check internet {:?}", msg);
            match msg {
                CheckInternetOutput::BackPressed => Message::ChangeScreen(Pages::StartScreen),
                CheckInternetOutput::LinkMachine => Message::ChangeScreen(Pages::LinkMachine),
                CheckInternetOutput::ConnectionNotFound => Message::ChangeScreen(Pages::NoInternet),
                CheckInternetOutput::ShowError(error) => {
                    Message::ChangeScreen(Pages::SetupFailed(error, "check_internet".to_string()))
                }
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
                LinkMachineOutput::ShowError => Message::ChangeScreen(Pages::SetupFailed("".to_owned(), "".to_owned())),
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
            ConfigureOutput::SetupSuccess(machine_info) =>  Message::ChangeScreen(Pages::SetupSuccess(machine_info)),
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
                Message::ChangeScreen(Pages::MachineInfo)
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
                println!("REFRESH SCREEN: {:?}", screen.to_owned());

                match screen {
                    screen if screen == String::from("check_internet") =>  Message::ChangeScreen(Pages::CheckInternet),
                    screen if screen == String::from("configure_machine") =>  Message::ChangeScreen(Pages::ConfigureMachine),
                    _ => {
                        println!("Found something else");
                    Message::ChangeScreen(Pages::MachineInfo)},
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
                DevicePageOutput::Exit => Message::Exit,
            }),
        );
    machine_info
}

#[tokio::main]
async fn main() {
    let app = RelmApp::new("mecha.connect.app");

    let settings = match settings::read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => ScreenSettings::default(),
    };

    let css = settings.css.clone();
    app.set_global_css_from_file(css.default);

    app.run_async::<MechaConnectApp>(());
}
