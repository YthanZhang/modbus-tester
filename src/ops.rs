use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;

use iced::{
    alignment::{Horizontal, Vertical},
    widget::{Button, Column, PickList, Row, Scrollable, Text, TextInput},
    Alignment, Command, Element, Length,
};

use serde::{Deserialize, Serialize};

use crate::message_sender::Operation;


/// Type of available operations without operation info
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Copy, Clone)]
pub enum OpType {
    ReadSingle,
    WriteSingle,
    ReadSingleRO,
}

const OP_TYPE_ALL: &[OpType] =
    &[OpType::ReadSingle, OpType::WriteSingle, OpType::ReadSingleRO];

impl Display for OpType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                OpType::ReadSingle => {
                    "Read Single"
                }
                OpType::WriteSingle => {
                    "Write Single"
                }
                OpType::ReadSingleRO => {
                    "Read Single RO"
                }
            }
        )
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct OpView {
    pub(crate) name: String,
    pub(crate) op_type: OpType,
    pub(crate) op_addr: String,
    pub(crate) op_val: String,
    pub(crate) eval_str: String,
}

impl OpView {
    pub fn new(
        name: String,
        op_type: OpType,
        op_addr: String,
        op_val: String,
        eval_str: String,
    ) -> Self {
        Self { name, op_type, op_addr, op_val, eval_str }
    }

    pub fn view(&self) -> Element<OpViewMessage> {
        Row::new()
            .width(Length::FillPortion(10))
            .align_items(Alignment::Center)
            .push(
                TextInput::new("Name", &self.name, OpViewMessage::SetName)
                    .width(Length::FillPortion(15))
                    .padding([0, 2]),
            )
            .push(
                PickList::new(
                    OP_TYPE_ALL,
                    Some(self.op_type),
                    OpViewMessage::SelectOpType,
                )
                .width(Length::Units(150))
                .padding([0, 2]),
            )
            .push({
                let row = Row::new()
                    .width(Length::FillPortion(30))
                    .align_items(Alignment::Center)
                    .push(
                        TextInput::new(
                            "Address",
                            &self.op_addr,
                            OpViewMessage::SetOpAddr,
                        )
                        .width(Length::Fill)
                        .padding([0, 2]),
                    );

                if self.op_type == OpType::WriteSingle {
                    row.push(
                        TextInput::new(
                            "Value",
                            &self.op_val,
                            OpViewMessage::SetOpValue,
                        )
                        .width(Length::Fill)
                        .padding([0, 2]),
                    )
                } else {
                    row
                }
            })
            .push(
                TextInput::new(
                    "Value Conversion",
                    &self.eval_str,
                    OpViewMessage::SetEval,
                )
                .width(Length::FillPortion(25))
                .padding([0, 2]),
            )
            .push(
                Button::new(
                    Text::new("Send")
                        .vertical_alignment(Vertical::Center)
                        .horizontal_alignment(Horizontal::Center)
                        .size(20),
                )
                .on_press(OpViewMessage::SendRequest(self.clone()))
                .width(Length::FillPortion(8))
                .padding([0, 2]),
            )
            .into()
    }

    pub fn update(&mut self, message: OpViewMessage) -> Command<OpViewMessage> {
        match message {
            OpViewMessage::SetName(val) => {
                self.name = val;
                Command::none()
            }
            OpViewMessage::SelectOpType(op_type) => {
                self.op_type = op_type;
                Command::none()
            }
            OpViewMessage::SetOpAddr(val) => {
                self.op_addr = val;
                Command::none()
            }
            OpViewMessage::SetOpValue(val) => {
                self.op_val = val;
                Command::none()
            }
            OpViewMessage::SetEval(val) => {
                self.eval_str = val;
                Command::none()
            }
            OpViewMessage::SendRequest(_) => {
                unreachable!();
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum OpViewMessage {
    SetName(String),
    SelectOpType(OpType),
    SetOpAddr(String),
    SetOpValue(String),
    SetEval(String),
    SendRequest(OpView),
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize)]
pub struct OpViewList {
    ops: Vec<OpView>,
}

impl Deref for OpViewList {
    type Target = Vec<OpView>;

    fn deref(&self) -> &Self::Target {
        &self.ops
    }
}

impl TryFrom<OpViewList> for Vec<Operation> {
    type Error = crate::error::Error;

    fn try_from(value: OpViewList) -> Result<Self, Self::Error> {
        value.ops.into_iter().map(|op| op.try_into()).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpViewListMessage {
    AddOperation,
    RemoveOperation(usize),
    OpViewMessage(usize, OpViewMessage),
    SendRequest(OpView),
}

impl OpViewList {
    pub fn view(&self) -> Element<OpViewListMessage> {
        let mut column =
            Column::new().width(Length::FillPortion(50)).height(Length::Shrink);

        for (idx, op) in self.ops.iter().enumerate() {
            column = column.push(
                Row::new()
                    .padding(5)
                    .align_items(Alignment::Center)
                    .width(Length::Fill)
                    .push(
                        Button::new(
                            Text::new("-")
                                .vertical_alignment(Vertical::Center)
                                .horizontal_alignment(Horizontal::Center)
                                .size(20),
                        )
                        .on_press(OpViewListMessage::RemoveOperation(idx)),
                    )
                    .push(op.view().map(move |msg| {
                        if let OpViewMessage::SendRequest(op_view) = msg {
                            OpViewListMessage::SendRequest(op_view)
                        } else {
                            OpViewListMessage::OpViewMessage(idx, msg)
                        }
                    })),
            );
        }

        column = column.push(
            Row::new()
                .push(
                    Button::new(
                        Text::new("+")
                            .vertical_alignment(Vertical::Center)
                            .horizontal_alignment(Horizontal::Center),
                    )
                    .width(Length::Fill)
                    .on_press(OpViewListMessage::AddOperation),
                )
                .padding(5),
        );

        Scrollable::new(column).into()
    }

    pub fn update(
        &mut self,
        message: OpViewListMessage,
    ) -> Command<OpViewListMessage> {
        match message {
            OpViewListMessage::AddOperation => {
                self.ops.push(OpView::new(
                    self.ops.len().to_string(),
                    OpType::ReadSingle,
                    "".to_string(),
                    "".to_string(),
                    "val".to_string(),
                ));
                Command::none()
            }
            OpViewListMessage::RemoveOperation(idx) => {
                self.ops.remove(idx);
                Command::none()
            }
            OpViewListMessage::OpViewMessage(idx, msg) => self.ops[idx]
                .update(msg)
                .map(move |msg| OpViewListMessage::OpViewMessage(idx, msg)),
            OpViewListMessage::SendRequest(_) => {
                unreachable!()
            }
        }
    }
}
