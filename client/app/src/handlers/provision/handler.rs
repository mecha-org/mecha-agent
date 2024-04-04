use std::thread;
use std::time::Instant;
use std::{io::Error, time::Duration};
use anyhow::{bail, Result};
use relm4::Sender;
use tokio::time::sleep;
use tokio::{select, sync::mpsc, time};
use crate::server::provision_client::ProvisionManagerClient;

use crate::pages::link_machine::InputMessage as Message;

#[derive(Debug)]
enum HandleMessage {
    GenerateCodeRes { response: Result<String> },
    ProvisionCodeRes { response: Result<bool> },
    TimeoutCodeRes { response: Result<f64> },
}

pub struct LinkMachineHandler {
}

impl LinkMachineHandler {

    pub fn new() -> Self {
        Self { 
        }
    }

    pub async fn run(&mut self, sender: Sender<Message>) -> Result<(), Error> {

        let (event_tx, mut event_rx) = mpsc::channel(128);

        let (g_code_message_tx, g_code_message_rx) = mpsc::channel(128);
        let mut g_code_handler = GenerateCodeHandler::new(event_tx.clone());

        let g_code_t = tokio::spawn(async move {
            g_code_handler.run(g_code_message_rx).await;
        });

        
        let (p_code_message_tx, p_code_message_rx) = mpsc::channel(128);
        let mut p_code_handler = ProvisionCodeHandler::new(event_tx.clone());
        
        let p_code_t = tokio::spawn(async move {
            p_code_handler.run(p_code_message_rx).await;
        });


        // let (t_code_message_tx, t_code_message_rx) = mpsc::channel(128);
        // let mut t_code_handler = TimeoutCodeHandler::new(event_tx);
        
        // let t_code_t = tokio::spawn(async move {
        //     t_code_handler.run(t_code_message_rx).await;
        // });

        let _ = sender.send(Message::ProvisioningTasks { 
            g: g_code_t, 
            p: p_code_t,
            // t: t_code_t 
        });


        let mut process_time: f64 = 1.0;

        loop {
            select! {
                    event = event_rx.recv() => {
                        if event.is_none() {
                            continue;
                        }

                        match event.unwrap() {
                            HandleMessage::GenerateCodeRes { response } => {
                                match response {
                                    Ok(code) => {
                                        let _ = p_code_message_tx.send(PCodeHandlerMessage::CodeChanged { code: code.clone() }).await;
                                        let _ = sender.send(Message::CodeChanged(code.clone()));

                                        // let _ = t_code_message_tx.send(TCodeHandlerMessage::CodeChanged { code: code.clone() }).await;

                                        // let mut target_value = 1.0 as f64;  
                                        // println!("remaining time : {} ", target_value-0.1);
                                        // let _ = sender.send(Message::UpdateTimer(0.9983)); 

                                        // let mut interval = time::interval(time::Duration::from_secs(1)); 
                                        //     while process_time > 0.0 {
                                        //         interval.tick().await;
                                        //         process_time -= 0.01;
                                        //         println!("fraction_value {:?} ", process_time.to_owned());
                                        //         let _ = sender.send(Message::UpdateTimer(process_time.to_owned())); 
                                        //     }
                                        // process_time = 1.0;

                                    }, 
                                    Err(e) => {
                                        let _ = sender.send(Message::GenerateCodeError("Error".to_owned()));
                                    }
                                }
                            }
                            HandleMessage::TimeoutCodeRes { response } =>  {
                                println!("TimeoutCodeRes res event {:?}", response);
                                match response {
                                    Ok(value) => {
                                        println!("timeout value {:?}  ", value.clone());
                                        let _ = sender.send(Message::UpdateTimer(value)); 
                                    },
                                    Err(e) => {
                                        println!("error in tcode {}", e);
                                        let _ = sender.send(Message::GenerateCodeError("Error".to_owned()));
                                    }
                                }
                            } 
                            HandleMessage::ProvisionCodeRes { response } => {
                                match response {
                                    Ok(success) => {
                                        if success {
                                            let _ = g_code_message_tx.send(GCodeHandlerMessage::ChangeRunningStatus { status: RunningStatus::STOP }).await;
                                            let _ = p_code_message_tx.send(PCodeHandlerMessage::ChangeRunningStatus { status: RunningStatus::STOP }).await;
                                            // let _ = t_code_message_tx.send(TCodeHandlerMessage::ChangeRunningStatus { status: RunningStatus::STOP }).await;

                                            let _ = sender.send(Message::ProvisionSuccess);

                                        }
                                    },
                                    Err(e) => {
                                        let _ = sender.send(Message::ProvisionSuccess);
                                    }
                                }
                            }
                        };
                    }
            }
        }
        // g_code_t.await.unwrap();
        // p_code_t.await.unwrap();

    }
  
}
  

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RunningStatus {
    INACTIVE,
    START,
    STOP,
}

pub enum GCodeHandlerMessage {
    ChangeRunningStatus { status: RunningStatus },
}
struct GenerateCodeHandler {
    is_calling: bool,
    status: RunningStatus,
    event_tx: mpsc::Sender<HandleMessage>,
}

impl GenerateCodeHandler {
    pub fn new(event_tx: mpsc::Sender<HandleMessage>) -> Self {
        Self {
            is_calling: false,
            status: RunningStatus::START,
            event_tx,
        }
    }

    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<GCodeHandlerMessage>) {
        let mut g_code_interval = time::interval(Duration::from_secs(60));

        loop {
            select! {
                    _ = g_code_interval.tick() => {

                        if !self.is_calling && self.status == RunningStatus::START {
                            self.is_calling = true;
                            let generate_code_response = g_code().await;

                            match generate_code_response {
                                Ok(response) => {
                                    let _ = self.event_tx.send(HandleMessage::GenerateCodeRes {response: Ok(response.code.clone()) }).await;
                                    self.is_calling = false;
                                }
                                Err(e) => {
                                    eprintln!("Error in generate code : {:?} ", e);
                                }
                            }
                           
                        } 
                    }
                    msg = message_rx.recv() => {
                        if msg.is_none() {
                            continue;
                        }

                        match msg.unwrap() {
                            GCodeHandlerMessage::ChangeRunningStatus { status } => {
                                self.status = status;
                                if status.clone() != RunningStatus::STOP { println!("continue!") };
                                break;
                            }
                        };
                    }
            }
        }
    }
}



pub enum TCodeHandlerMessage { 
    ChangeRunningStatus { status: RunningStatus },
    CodeChanged { code: String },
}
struct TimeoutCodeHandler {
    is_calling: bool,
    status: RunningStatus,
    code: Option<String>,
    event_tx: mpsc::Sender<HandleMessage>,
}

impl TimeoutCodeHandler {
    pub fn new(event_tx: mpsc::Sender<HandleMessage>) -> Self {
        Self {
            is_calling: false,
            status: RunningStatus::START,
            code: None,
            event_tx,
        }
    }

    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<TCodeHandlerMessage>) {
        let mut t_code_interval = time::interval(Duration::from_secs(1));
        let mut process_time: f64 = 1.0;

        loop {
            select! {
                    _ = t_code_interval.tick() => {

                        if !self.is_calling && self.status == RunningStatus::START && self.code.is_some(){

                            self.is_calling = true;

                            process_time -= 0.01;
                            println!("fraction_value {:?} ", process_time.to_owned());
                            let _ = self.event_tx.send(HandleMessage::TimeoutCodeRes { response : Ok(process_time.to_owned())}).await; 
                            self.is_calling = false;

                            if process_time <= 0.00 { process_time = 1.0;}
                        }
                    }
                    msg = message_rx.recv() => {
                        if msg.is_none() {
                            continue;
                        }

                        match msg.unwrap() {
                            TCodeHandlerMessage::CodeChanged {code} => {
                                self.code = Some(code.clone());
                            }
                            TCodeHandlerMessage::ChangeRunningStatus { status } => {
                                self.status = status;
                                if status.clone() != RunningStatus::STOP { println!("continue!") };
                                break;
                            }
                        };
                    }
            }
        }
    }
}




pub enum PCodeHandlerMessage {
    ChangeRunningStatus { status: RunningStatus },
    CodeChanged { code: String },
}

struct ProvisionCodeHandler {
    is_calling: bool,
    code: Option<String>,
    status: RunningStatus,
    event_tx: mpsc::Sender<HandleMessage>,
}

impl ProvisionCodeHandler {
    pub fn new(event_tx: mpsc::Sender<HandleMessage>) -> Self {
        Self {
            is_calling: false,
            status: RunningStatus::START,
            code: None,
            event_tx,
        }
    }

    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<PCodeHandlerMessage>) {
        let mut p_code_interval = time::interval(Duration::from_secs(10));
        loop {
            select! {
                    _ = p_code_interval.tick() => {

                        if !self.is_calling && self.status == RunningStatus::START && self.code.is_some(){
                            self.is_calling = true;
                            let provisioning_res = p_code(self.code.clone().unwrap()).await;

                            match provisioning_res {
                                Ok(response) => {

                                    if response.success.clone() {
                                        let _ = self.event_tx.send(HandleMessage::ProvisionCodeRes {response: Ok(response.success.clone()) }).await;
                                        self.is_calling = true;
                                        break;
                                    }
                                    else {
                                        self.is_calling = false;
                                    }
                                },
                                Err(e) => { 
                                    self.is_calling = false;
                                }
                            }

                        }
                    }
                    msg = message_rx.recv() => {
                        if msg.is_none() {
                            continue;
                        }

                        match msg.unwrap() {
                            PCodeHandlerMessage::ChangeRunningStatus { status } => {
                                self.status = status;
                            }
                            PCodeHandlerMessage::CodeChanged {code} => {
                                self.code = Some(code);
                            }
                        };
                    }

            }
        }
    }
}


#[derive(Debug)]
pub struct GenerateCodeResp {
    pub code: String,
    pub message: String
}

pub async fn g_code() -> anyhow::Result<GenerateCodeResp> {
    let provision_manager_client_response = ProvisionManagerClient::new().await;
    let mut provision_manager_client = match provision_manager_client_response {
        Ok(r) => r,
        Err(e) => {
            bail!("Provision Handler-connect clinet error:: {}", e);
        }
    };

    let generate_code_response = provision_manager_client.generate_code().await;
    let provisioning_code: GenerateCodeResp = match generate_code_response {
        Ok(r) => {
            GenerateCodeResp {
                code: r.code,
                message: String::from("")
            }
        },
        Err(e) => {
            eprintln!("Provision Handler-generate_code error:: {:?}", e);
            GenerateCodeResp {
                code: String::from(""),
                message: e.to_string()
            }
        }
    };

    Ok(provisioning_code)
}

#[derive(Debug)]
pub struct ProvisioningStatusResponse {
    pub success: bool,
    pub message: String
}

pub async fn p_code(code: String) -> anyhow::Result<ProvisioningStatusResponse>  {
    let provision_manager_client_response = ProvisionManagerClient::new().await;
    let mut provision_manager_client = match provision_manager_client_response {
        Ok(r) => r,
        Err(e) => {
            bail!("Provision Handler-connect clinet error:: {}", e);
        }
    };

    let provisioning_response: ProvisioningStatusResponse = match provision_manager_client.provision_by_code(code).await {
        Ok(r) => {
            ProvisioningStatusResponse {
                success: true,
                message: String::from("")

            }
        },
        Err(e) => {
            ProvisioningStatusResponse {
                success: false,
                message: e.to_string()
            }
        }
    };
    Ok(provisioning_response)

}