use std::fmt::Debug;
use std::str::FromStr;

use meval::Expr;

use crate::error::*;
use crate::ops::*;
use crate::port_op::PortConfig;
use crate::string_to_num::ParseNum;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Request {
    ReadSingle(u16),
    WriteSingle(u16, f64, u16),
    ReadSingleRO(u16),
}

impl Request {
    pub fn variant_string(&self) -> String {
        match self {
            Request::ReadSingle(_) => "ReadSingle".to_string(),
            Request::WriteSingle(_, _, _) => "WriteSingle".to_string(),
            Request::ReadSingleRO(_) => "ReadSingleRO".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Operation {
    pub name: String,
    pub req: Request,
    eval_str: String,
}

impl TryFrom<OpView> for Operation {
    type Error = Error;

    fn try_from(value: OpView) -> Result<Self, Self::Error> {
        let eval_func = match Expr::from_str(&value.eval_str) {
            Ok(eval) => match eval.bind("val") {
                Ok(func) => func,
                Err(_) => {
                    return Err(Error::with_message(
                        ErrKind::MathOperationParseError,
                        "Expression must contain \"val\"".to_string(),
                    ))
                }
            },
            Err(_) => {
                return Err(Error::with_message(
                    ErrKind::MathOperationParseError,
                    format!(
                        "Could not parse \"{}\" into valid math expression",
                        value.eval_str
                    ),
                ))
            }
        };

        let op_addr = match value.op_addr.parse_num::<u16>() {
            Ok(addr) => addr,
            Err(_) => {
                return Err(Error::with_message(
                    ErrKind::RequestParseError,
                    format!(
                        "\"{}\" is no a valid register address",
                        value.op_addr
                    ),
                ))
            }
        };

        let req = {
            match value.op_type {
                OpType::ReadSingle => Request::ReadSingle(op_addr),
                OpType::WriteSingle => {
                    let val = match value.op_val.parse_num::<f64>() {
                        Ok(val) => val,
                        Err(_) => {
                            return Err(Error::with_message(
                                ErrKind::RequestParseError,
                                format!(
                                    "\"{}\" is no a valid register value",
                                    value.op_val
                                ),
                            ))
                        }
                    };

                    let eval_val = eval_func(val).round();
                    if eval_val < 0f64 || eval_val > u16::MAX as f64 {
                        return Err(Error::with_message(
                                ErrKind::MathOperationResultInOutOfRangeValue,
                                format!("{} cannot be evaluated to a value in the range [0, 0xFFFF]", value.op_val))
                            );
                    }

                    Request::WriteSingle(op_addr, val, eval_val as u16)
                }
                OpType::ReadSingleRO => Request::ReadSingleRO(op_addr),
            }
        };

        Ok(Self { name: value.name, req, eval_str: value.eval_str })
    }
}

impl Operation {
    pub fn get_eval(&self) -> Box<dyn Fn(f64) -> f64> {
        // self.eval_str should have been checked in operation creation
        // so here it is guaranteed to be valid
        Box::new(Expr::from_str(&self.eval_str).unwrap().bind("val").unwrap())
    }

    pub fn to_modbus_bytes(&self, port_conf: &PortConfig) -> [u8; 8] {
        const CRC_GEN: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_MODBUS);

        let mut req_bytes: [u8; 8] =
            [port_conf.device_addr, 0, 0, 0, 0, 0, 0, 0];

        let (addr, val) = match self.req {
            Request::ReadSingle(addr) => {
                req_bytes[1] = 0x03;
                (addr, 1)
            }
            Request::WriteSingle(addr, _original, val) => {
                req_bytes[1] = 0x06;
                (addr, val)
            }
            Request::ReadSingleRO(addr) => {
                req_bytes[1] = 0x04;
                (addr, 1)
            }
        };

        req_bytes[2] = (addr >> 8) as u8;
        req_bytes[3] = addr as u8;
        req_bytes[4] = (val >> 8) as u8;
        req_bytes[5] = val as u8;

        let crc = CRC_GEN.checksum(&req_bytes[..6]);
        req_bytes[6] = crc as u8;
        req_bytes[7] = (crc >> 8) as u8;

        req_bytes
    }
}
