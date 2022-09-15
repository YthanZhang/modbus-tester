use std::fmt::{Display, Formatter};
use std::io::Write;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::read_to_timeout::ReadToTimeout;
use crate::string_to_num::ParseNum;

use crate::error::{ErrKind, Error};
use crate::message_sender::{Operation, Request};
use crate::{OpView, OpViewList};


pub const PARITIES: &[Parity] = &[Parity::None, Parity::Odd, Parity::Even];
pub const STOP_BITS: &[StopBits] = &[StopBits::One, StopBits::Two];


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Parity {
    None,
    Odd,
    Even,
}

impl From<Parity> for serialport::Parity {
    fn from(p: Parity) -> Self {
        match p {
            Parity::None => serialport::Parity::None,
            Parity::Odd => serialport::Parity::Odd,
            Parity::Even => serialport::Parity::Even,
        }
    }
}

impl Display for Parity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StopBits {
    One,
    Two,
}

impl From<StopBits> for serialport::StopBits {
    fn from(stop_bits: StopBits) -> Self {
        match stop_bits {
            StopBits::One => serialport::StopBits::One,
            StopBits::Two => serialport::StopBits::Two,
        }
    }
}

impl Display for StopBits {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortOption {
    pub port_name: Option<String>,
    pub baud: String,
    pub stop_bits: Option<StopBits>,
    pub parity: Option<Parity>,
    pub device_addr: String,
}

impl Default for PortOption {
    fn default() -> Self {
        Self {
            port_name: None,
            baud: "".to_string(),
            stop_bits: None,
            parity: None,
            device_addr: "".to_string(),
        }
    }
}

impl TryFrom<PortOption> for PortConfig {
    type Error = Error;

    fn try_from(option: PortOption) -> Result<Self, Self::Error> {
        if option.port_name.is_none()
            || option.baud.is_empty()
            || option.stop_bits.is_none()
            || option.parity.is_none()
        {
            return Err(Error::with_message(
                ErrKind::InvalidPortOption,
                "Must select all port options".to_string(),
            ));
        }

        let baud = match option.baud.parse_num::<u32>() {
            Ok(baud) => baud,
            Err(_) => {
                return Err(Error::with_message(
                    ErrKind::InvalidPortOption,
                    format!("\"{}\" is not a valid baud", option.baud),
                ));
            }
        };

        let device_addr = match option.device_addr.parse_num::<u8>() {
            Ok(addr) => addr,
            Err(_) => {
                return Err(Error::with_message(
                    ErrKind::InvalidPortOption,
                    format!(
                        "\"{}\" is not a valid device address",
                        option.device_addr
                    ),
                ))
            }
        };

        // These unwraps were already checked
        Ok(Self {
            port_name: option.port_name.unwrap(),
            baud,
            stop_bits: option.stop_bits.unwrap().into(),
            parity: option.parity.unwrap().into(),
            device_addr,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortConfig {
    pub port_name: String,
    pub baud: u32,
    pub stop_bits: serialport::StopBits,
    pub parity: serialport::Parity,
    pub device_addr: u8,
}

impl Default for PortConfig {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud: 0,
            stop_bits: serialport::StopBits::One,
            parity: serialport::Parity::None,
            device_addr: 0,
        }
    }
}

impl PortConfig {
    pub fn new(
        port_name: String,
        baud: u32,
        stop_bits: StopBits,
        parity: Parity,
        device_addr: u8,
    ) -> Self {
        let parity = parity.into();
        let stop_bits = stop_bits.into();
        PortConfig { port_name, baud, stop_bits, parity, device_addr }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Response {
    pub op: Operation,
    bytes: Vec<u8>,
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        const CRC_GEN: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_MODBUS);

        fn make_msg(
            f: &mut Formatter<'_>,
            req: Request,
            name: &str,
            ret: &str,
            bytes: &[u8],
        ) -> std::fmt::Result {
            let addr = match req {
                Request::ReadSingle(addr) => addr,
                Request::WriteSingle(addr, _, _) => addr,
                Request::ReadSingleRO(addr) => addr,
            };

            write!(
                f,
                "{:?}: {}(0x{:02X}) -> {}: ",
                req.variant_string(),
                name,
                addr,
                ret,
            )?;

            let mut iter = bytes.iter();
            write!(f, "{{ ")?;
            if let Some(byte) = iter.next() {
                write!(f, " {:02X}", byte)?;

                for byte in iter {
                    write!(f, " {:02X}", byte)?;
                }
            }
            write!(f, " }}")?;

            Ok(())
        }

        if self.bytes.len() < 5 {
            return make_msg(
                f,
                self.op.req,
                &self.op.name,
                "!InvalidResponse",
                &self.bytes,
            );
        }

        let msg_crc = (self.bytes[self.bytes.len() - 2] as u16)
            | ((self.bytes[self.bytes.len() - 1] as u16) << 8);
        if CRC_GEN.checksum(&self.bytes[0..(self.bytes.len() - 2)]) != msg_crc {
            return make_msg(
                f,
                self.op.req,
                &self.op.name,
                "!CRCCheckFailed",
                &self.bytes,
            );
        }

        let make_u16 = |msb, lsb| ((msb as u16) << 8) | lsb as u16;
        let (_addr, value) = match self.op.req {
            Request::ReadSingle(addr) | Request::ReadSingleRO(addr) => {
                if self.bytes.len() != 7 {
                    (addr, "!UnexpectedResponse".to_string())
                } else {
                    (
                        addr,
                        (*self.op.get_eval())(make_u16(
                            self.bytes[3],
                            self.bytes[4],
                        ) as f64)
                        .to_string(),
                    )
                }
            }
            Request::WriteSingle(addr, original, _val) => {
                if self.bytes.len() != 8 {
                    (addr, "!UnexpectedResponse".to_string())
                } else {
                    (addr, original.to_string())
                }
            }
        };

        make_msg(f, self.op.req, &self.op.name, &value, &self.bytes)
    }
}

impl Response {
    fn new(op: Operation, bytes: Vec<u8>) -> Self {
        Self { op, bytes }
    }
}

pub async fn one_shot_quarry(
    op: OpView,
    port_option: PortOption,
    port_op_tx: Sender<OpMessage>,
) -> Result<Response, Error> {
    let op: Operation = op.try_into()?;
    let port_conf: PortConfig = port_option.try_into()?;

    let (response_tx, response_rx) = channel();

    if port_op_tx.send(OpMessage::OneShot(port_conf, op, response_tx)).is_err() {
        return Err(Error::new(ErrKind::PortOpThreadNotPresent));
    }

    if let Ok(result) = response_rx.recv() {
        result
    } else {
        Err(Error::new(ErrKind::PortOpDroppedChannelTxWithoutResponse))
    }
}

pub async fn continuous_quarry_start(
    op_list: OpViewList,
    port_option: PortOption,
    port_op_tx: Sender<OpMessage>,
    sender: Sender<Result<Response, Error>>,
) -> Result<(), Error> {
    let op_list = op_list.try_into()?;
    let port_conf = port_option.try_into()?;

    if port_op_tx
        .send(OpMessage::StartContinuous(port_conf, op_list, sender))
        .is_err()
    {
        Err(Error::new(ErrKind::PortOpThreadNotPresent))
    } else {
        Ok(())
    }
}

pub async fn continuous_quarry_get_results(
    rx: Arc<Mutex<Receiver<Result<Response, Error>>>>,
) -> Result<Vec<Result<Response, Error>>, Error> {
    // Locking really shouldn't fail, crash the process if that happens
    let rx = rx.lock().unwrap();
    let response = if let Ok(response) = rx.recv() {
        response
    } else {
        return Err(Error::with_message(
            ErrKind::PortOpThreadNotPresent,
            "port op thread not present".to_string(),
        ));
    };

    let mut result = vec![response];

    while let Ok(response) = rx.try_recv() {
        result.push(response);
    }

    Ok(result)
}

pub async fn continuous_quarry_stop(tx: Sender<OpMessage>) {
    let _ = tx.send(OpMessage::StopContinuous);
}

/// Message to control port operations on port_op_thread
/// This message should be send through mpsc channel
pub enum OpMessage {
    OneShot(PortConfig, Operation, Sender<Result<Response, Error>>),
    StartContinuous(PortConfig, Vec<Operation>, Sender<Result<Response, Error>>),
    StopContinuous,
}

pub fn port_op_thread(rx: Receiver<OpMessage>) -> ! {
    let mut op_queue = vec![];

    loop {
        op_queue.clear();
        // There should always be a sender present, if not panic
        let (port_conf, response_tx, continuous) = match rx.recv().unwrap() {
            OpMessage::OneShot(port_conf, op, tx) => {
                op_queue.push(op);
                (port_conf, tx, false)
            }
            OpMessage::StartContinuous(port_conf, ops, tx) => {
                if ops.is_empty() {
                    continue;
                }
                op_queue = ops;
                (port_conf, tx, true)
            }
            OpMessage::StopContinuous => {
                continue;
            }
        };

        // open port, if failed, send error back through response_tx
        let mut port =
            match serialport::new(port_conf.port_name.clone(), port_conf.baud)
                .parity(port_conf.parity)
                .stop_bits(port_conf.stop_bits)
                .timeout(Duration::from_millis(50))
                .open()
            {
                Ok(port) => port,
                Err(_) => {
                    // don't care if send fails because response_tx is dropped
                    // after continue
                    let _ = response_tx.send(Err(Error::with_message(
                        ErrKind::FailedToOpenTargetPort,
                        format!(
                            "Failed to open port \"{}\"",
                            port_conf.port_name
                        ),
                    )));
                    continue;
                }
            };

        let mut iter = op_queue.iter();
        loop {
            let recv_result = rx.try_recv(); // must bind to longer life time
            let (req, response_tx, extra_oneshot) = if let Ok(op_msg) =
                &recv_result
            {
                match op_msg {
                    OpMessage::OneShot(new_port_conf, op, resp_tx) => {
                        if *new_port_conf != port_conf {
                            // don't care if the send fails
                            let _ = resp_tx.send(Err(Error::with_message(
                                ErrKind::PortTypeUnequal,
                                "The latest one shot query request is using a different \
                                port config, please stop current continuous quarry".to_string()
                            )));
                            continue;
                        } else {
                            (op, resp_tx, true)
                        }
                    }
                    OpMessage::StartContinuous(_, _, resp_tx) => {
                        // don't care if the send fails
                        let _ = resp_tx.send(Err(Error::with_message(
                            ErrKind::AttemptToStartMultipleContinuousQuarry,
                            "Cannot start a new continuous quarry before stopping \
                            the old continuous quarry request".to_string(),
                        )));
                        continue;
                    }
                    OpMessage::StopContinuous => {
                        break;
                    }
                }
            } else {
                match iter.next() {
                    Some(req) => (req, &response_tx, false),
                    None => {
                        // None case only happens in continuous quarry
                        iter = op_queue.iter();

                        // unwrap because there's no way for a new op_queue iter to be empty
                        (iter.next().unwrap(), &response_tx, false)
                    }
                }
            };

            if let Err(e) = port.write_all(&req.to_modbus_bytes(&port_conf)) {
                // don't care if send failed because response_tx is dropped after break
                let _ = response_tx.send(Err(Error::with_message(
                    ErrKind::PortWriteFailed,
                    format!("Failed to write msg to port due to: {}", e),
                )));
                break;
            }

            let mut response = Vec::new();
            let _ = port.read_to_timeout(&mut response);

            if response_tx
                .send(Ok(Response::new(req.clone(), response)))
                .is_err()
            {
                break;
            }

            if !continuous && !extra_oneshot {
                break;
            }
            std::thread::sleep(Duration::from_millis(40));
        }
    }
}
