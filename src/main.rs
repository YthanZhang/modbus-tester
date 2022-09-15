extern crate core;

mod error;
mod message_sender;
mod ops;
mod port_op;
mod response_display;

use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};

use iced::{
    alignment::Vertical,
    widget::{
        scrollable, Button, Column, Container, PickList, Row, Space, TextInput,
    },
    Application, Command, Element, Length, Settings,
};

use serde::{Deserialize, Serialize};

use crate::error::*;
use crate::ops::*;
use crate::port_op::*;
use crate::response_display::*;

/**
Entry point
*/
fn main() -> iced::Result {
    let mut setting = Settings::with_flags(());
    setting.window = iced::window::Settings {
        size: (1280, 720),
        position: Default::default(),
        min_size: Some((1280, 720)),
        max_size: None,
        visible: true,
        resizable: true,
        decorations: true,
        transparent: false,
        always_on_top: false,
        icon: None,
    };

    setting.default_font = Some(include_bytes!("../JetBrainsMono-Regular.ttf"));

    App::run(setting)
}

#[derive(Debug, PartialEq, Clone)]
enum Message {
    None,

    OneShotViewList(OpViewListMessage),
    ContinuousViewList(OpViewListMessage),
    OneShotDisplay(ResponseViewMessage),

    SaveLayout,
    RefreshAvailablePorts,
    SetComPort(String),
    SetParity(Parity),
    SetStopBits(StopBits),
    SetBaud(String),
    SetDeviceAddress(String),

    OneShotQuarry(OpView),
    OneShotResponse(Result<Response, Error>),

    ContinuousQuarryToggle(OpViewList),
    ContinuousQuarryStartResult(Result<(), Error>),
    ContinuousQuarryResult(Result<Vec<Result<Response, Error>>, Error>),
}

#[derive(Serialize, Deserialize, Default)]
struct App {
    one_shot_ops: OpViewList,
    continuous_ops: OpViewList,

    #[serde(skip)]
    available_ports: Vec<String>,

    #[serde(skip)]
    port_option: PortOption,

    #[serde(skip)]
    responses: ResponseView,
    #[serde(skip)]
    continuous_responses: KeyedResponseView,

    #[serde(skip)]
    port_thread_sender: Option<Sender<OpMessage>>,

    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    continuous_quarry_channel:
        Option<Arc<Mutex<Receiver<Result<Response, Error>>>>>,
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let mut app = match std::fs::read_to_string("layout.ron") {
            Ok(string) => {
                ron::from_str::<App>(&string).unwrap_or_else(|_| App::default())
            }
            Err(_) => App::default(),
        };

        app.available_ports = serialport::available_ports()
            .unwrap()
            .into_iter()
            .map(|port| port.port_name)
            .collect::<Vec<_>>();

        let (tx, rx) = channel();

        std::thread::spawn(move || port_op_thread(rx));

        app.port_thread_sender = Some(tx);

        (app, Command::none())
    }

    fn title(&self) -> String {
        "Counter App".to_string()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::None => Command::none(),
            Message::OneShotViewList(msg) => {
                self.one_shot_ops.update(msg).map(Message::OneShotViewList)
            }
            Message::ContinuousViewList(msg) => {
                self.continuous_ops.update(msg).map(Message::ContinuousViewList)
            }
            Message::OneShotDisplay(msg) => {
                self.responses.update(msg).map(Message::OneShotDisplay)
            }

            Message::SaveLayout => {
                if let Ok(string) = ron::to_string(self) {
                    // don't care if write failed
                    let _ = std::fs::write("layout.ron", string);
                }

                Command::none()
            }
            Message::RefreshAvailablePorts => {
                self.available_ports = serialport::available_ports()
                    .unwrap()
                    .into_iter()
                    .map(|port| port.port_name)
                    .collect::<Vec<_>>();
                if let Some(port_name) = &self.port_option.port_name {
                    if !self.available_ports.iter().any(|name| name == port_name)
                    {
                        self.port_option.port_name = None;
                    }
                }
                Command::none()
            }
            Message::SetComPort(port_name) => {
                self.available_ports = serialport::available_ports()
                    .unwrap()
                    .into_iter()
                    .map(|port| port.port_name)
                    .collect::<Vec<_>>();
                if self.available_ports.iter().any(|s| *s == port_name) {
                    self.port_option.port_name = Some(port_name)
                } else {
                    self.port_option.port_name = None
                };
                Command::none()
            }
            Message::SetParity(parity) => {
                self.port_option.parity = Some(parity);
                Command::none()
            }
            Message::SetBaud(baud) => {
                self.port_option.baud = baud;
                Command::none()
            }
            Message::SetStopBits(stop_bits) => {
                self.port_option.stop_bits = Some(stop_bits);
                Command::none()
            }
            Message::SetDeviceAddress(addr) => {
                self.port_option.device_addr = addr;
                Command::none()
            }

            Message::OneShotQuarry(op_view) => Command::perform(
                one_shot_quarry(
                    op_view,
                    self.port_option.clone(),
                    self.port_thread_sender.clone().unwrap(),
                ),
                Message::OneShotResponse,
            ),
            Message::OneShotResponse(response) => {
                self.responses
                    .update(ResponseViewMessage::AddResponse(response))
                    .map(Message::OneShotDisplay);
                scrollable::snap_to(scrollable::Id::new("RespView"), 1.0)
            }

            Message::ContinuousQuarryToggle(op_list) => {
                let (tx, rx) = channel();
                match self.continuous_quarry_channel {
                    None => {
                        self.continuous_quarry_channel
                            .replace(Arc::new(Mutex::new(rx)));
                        self.continuous_responses
                            .update(KeyedResponseViewMessage::ClearResponses);

                        Command::perform(
                            continuous_quarry_start(
                                op_list,
                                self.port_option.clone(),
                                self.port_thread_sender.clone().unwrap(),
                                tx,
                            ),
                            Message::ContinuousQuarryStartResult,
                        )
                    }
                    Some(_) => {
                        let _ = self.continuous_quarry_channel.take();

                        Command::perform(
                            continuous_quarry_stop(
                                self.port_thread_sender.clone().unwrap(),
                            ),
                            |()| Message::None,
                        )
                    }
                }
            }
            Message::ContinuousQuarryStartResult(start_result) => {
                if let Ok(()) = start_result {
                    if let Some(rx) = &self.continuous_quarry_channel {
                        Command::perform(
                            continuous_quarry_get_results(rx.clone()),
                            Message::ContinuousQuarryResult,
                        )
                    } else {
                        Command::none()
                    }
                } else {
                    Command::none()
                }
            }
            Message::ContinuousQuarryResult(results) => match &self
                .continuous_quarry_channel
            {
                None => Command::none(),

                Some(rx) => match results {
                    Ok(results) => {
                        for (key, val) in results.into_iter().filter_map(|r| {
                            r.map_or(None, |r| Some((r.op.name.clone(), r)))
                        }) {
                            self.continuous_responses.update(
                                KeyedResponseViewMessage::AddResponse(
                                    key,
                                    Ok(val),
                                ),
                            );
                        }
                        Command::perform(
                            continuous_quarry_get_results(rx.clone()),
                            Message::ContinuousQuarryResult,
                        )
                    }
                    Err(_) => Command::perform(
                        continuous_quarry_get_results(rx.clone()),
                        Message::ContinuousQuarryResult,
                    ),
                },
            },
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        Column::new()
            .push(
                // top bar options
                Row::new()
                    .height(Length::Units(40))
                    .padding([5, 10])
                    .push(
                        // Save layout button
                        Container::new(
                            Button::new("Save Layout")
                                .on_press(Message::SaveLayout),
                        )
                        .padding([0, 2]),
                    )
                    .push(
                        // refresh port button
                        Container::new(
                            Button::new("Refresh")
                                .on_press(Message::RefreshAvailablePorts),
                        )
                        .padding([0, 4, 0, 32]),
                    )
                    .push(
                        // Com port picker
                        Container::new(
                            PickList::new(
                                &self.available_ports,
                                self.port_option.port_name.clone(),
                                Message::SetComPort,
                            )
                            .placeholder("Port"),
                        )
                        .padding([0, 16, 0, 4]),
                    )
                    .push(
                        // Parity picker
                        Container::new(
                            PickList::new(
                                PARITIES,
                                self.port_option.parity,
                                Message::SetParity,
                            )
                            .placeholder("Parity"),
                        )
                        .padding([0, 16]),
                    )
                    .push(
                        // Stop bits picker
                        Container::new(
                            PickList::new(
                                STOP_BITS,
                                self.port_option.stop_bits,
                                Message::SetStopBits,
                            )
                            .placeholder("Stop Bits"),
                        )
                        .padding([0, 16]),
                    )
                    .push(
                        // Baud setting
                        Container::new(TextInput::new(
                            "Baud",
                            &self.port_option.baud,
                            Message::SetBaud,
                        ))
                        .padding([0, 16])
                        .height(Length::Fill)
                        .width(Length::Units(96))
                        .align_y(Vertical::Center),
                    )
                    .push(
                        // Device address setting
                        Container::new(TextInput::new(
                            "Address",
                            &self.port_option.device_addr,
                            Message::SetDeviceAddress,
                        ))
                        .padding([0, 16])
                        .height(Length::Fill)
                        .width(Length::Units(96))
                        .align_y(Vertical::Center),
                    )
                    .push(Space::new(Length::Units(16), Length::Fill))
                    .push(
                        // toggle quarry button
                        Container::new(
                            Button::new("Toggle Continuous Quarry").on_press(
                                Message::ContinuousQuarryToggle(
                                    self.continuous_ops.clone(),
                                ),
                            ),
                        )
                        .padding([0, 4, 0, 32]),
                    ),
            )
            .push(
                Row::new()
                    .padding([5, 10])
                    .push(
                        Column::new()
                            .padding([4, 0])
                            .push(
                                // One shot view
                                Container::new(self.one_shot_ops.view().map(
                                    |msg| {
                                        if let OpViewListMessage::SendRequest(
                                            op_view,
                                        ) = msg
                                        {
                                            Message::OneShotQuarry(op_view)
                                        } else {
                                            Message::OneShotViewList(msg)
                                        }
                                    },
                                ))
                                .height(Length::FillPortion(70)),
                            )
                            .push(
                                scrollable(
                                    self.responses
                                        .view()
                                        .map(Message::OneShotDisplay),
                                )
                                .height(Length::FillPortion(30))
                                .id(scrollable::Id::new("RespView")),
                            )
                            .width(Length::FillPortion(50)),
                    )
                    .push(
                        // Continuous view or continuous response view
                        Container::new(
                            // if channel not present, show cv
                            if self.continuous_quarry_channel.is_none() {
                                self.continuous_ops.view().map(|msg| {
                                    if let OpViewListMessage::SendRequest(
                                        op_view,
                                    ) = msg
                                    {
                                        Message::OneShotQuarry(op_view)
                                    } else {
                                        Message::ContinuousViewList(msg)
                                    }
                                })
                            } else {
                                // else show responses
                                scrollable::Scrollable::new(
                                    self.continuous_responses
                                        .view()
                                        .map(|_msg| Message::None),
                                )
                                .into()
                            },
                        )
                        .padding([4, 0])
                        .width(Length::FillPortion(50)),
                    ),
            )
            .into()
    }
}
