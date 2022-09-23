use std::collections::HashMap;
use std::fmt::Debug;

use iced::{
    widget::{Column, Text},
    Command, Element, Length,
};

use crate::error::Error;
use crate::port_op::Response;


#[derive(Debug, PartialEq, Clone)]
pub enum ResponseViewMessage {
    AddResponse(Result<Response, Error>),
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct ResponseView {
    responses: Vec<Result<Response, Error>>,
}

/// This impl block is View logic and Update logic
impl ResponseView {
    pub fn view(&self) -> Element<ResponseViewMessage> {
        let mut column =
            Column::new().height(Length::Shrink).width(Length::Fill);

        for resp in &self.responses {
            let text = match resp {
                Ok(resp) => Text::new(resp.to_string()),
                Err(err) => Text::new(err.to_string()),
            }
            .width(Length::Fill);

            column = column.push(text);
        }

        column.into()
    }

    pub fn update(
        &mut self,
        msg: ResponseViewMessage,
    ) -> Command<ResponseViewMessage> {
        match msg {
            ResponseViewMessage::AddResponse(response) => {
                self.responses.push(response);
                Command::none()
            }
        }
    }
}

pub enum KeyedResponseViewMessage {
    AddResponse(String, Result<Response, Error>),
    ClearResponses,
}

#[derive(Debug, Clone, Default)]
pub struct KeyedResponseView {
    quarries: HashMap<String, Result<Response, Error>>,
}

impl KeyedResponseView {
    pub fn update(
        &mut self,
        msg: KeyedResponseViewMessage,
    ) -> Command<KeyedResponseViewMessage> {
        use KeyedResponseViewMessage::*;
        match msg {
            AddResponse(key, response) => {
                self.quarries.insert(key, response);
            }
            ClearResponses => {
                self.quarries.clear();
            }
        }

        Command::none()
    }

    pub fn view(&self) -> Element<KeyedResponseViewMessage> {
        let mut column =
            Column::new().height(Length::Shrink).width(Length::Fill);

        for (key, resp) in self.quarries.iter() {
            column = match resp {
                Ok(resp) => column.push(Text::new(resp.to_string())),
                Err(err) => column.push(Text::new(format!("{}: {}", key, err))),
            }
        }

        column.into()
    }
}
