use crate::error::Error;
use std::{ffi::CStr, fs::File, io::Read, str::from_utf8};

fn slice_to_usize(slice: &[u8]) -> usize {
    if let Ok(str) = from_utf8(slice) {
        if let Ok(num) = str.parse::<usize>() {
            return num;
        }
    }
    unimplemented!("Could not parse {slice:?} into option value");
}

trait Serialise {
    fn serialise(&self) -> Vec<u8>;
}

pub struct Session {
    data: Vec<u8>,
}
impl Session {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TftpOption {
    TransferSize(usize),
    BlockSize(usize),
    WindowSize(usize),
}

impl Serialise for TftpOption {
    fn serialise(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let value = match self {
            TftpOption::TransferSize(value) => {
                bytes.extend_from_slice(TftpOption::TSIZE);
                value
            }
            TftpOption::BlockSize(value) => {
                bytes.extend_from_slice(TftpOption::BLKSIZE);
                value
            }
            TftpOption::WindowSize(value) => {
                bytes.extend_from_slice(TftpOption::WINDOWSIZE);
                value
            }
        };
        bytes.push(Self::NULL);
        bytes.extend_from_slice(value.to_string().as_bytes());
        bytes.push(Self::NULL);

        bytes
    }
}

impl TftpOption {
    const TSIZE: &[u8] = &[0x74, 0x73, 0x69, 0x7a, 0x65];
    const BLKSIZE: &[u8] = &[0x62, 0x6c, 0x6b, 0x73, 0x69, 0x7a, 0x65];
    const WINDOWSIZE: &[u8] = &[0x77, 0x69, 0x6e, 0x64, 0x6f, 0x77, 0x73, 0x69, 0x7a, 0x65];
    const END: &[u8] = &[];
    const NULL: u8 = 0x00;

    fn parse(data: &[u8]) -> Vec<TftpOption> {
        let mut options = Vec::with_capacity(5);
        let mut options_raw = data.split(|chr| *chr == Self::NULL);

        while let Some(option) = options_raw.next() {
            match option {
                Self::TSIZE => options.push(TftpOption::TransferSize(slice_to_usize(
                    options_raw.next().unwrap(),
                ))),
                Self::BLKSIZE => options.push(TftpOption::BlockSize(slice_to_usize(
                    options_raw.next().unwrap(),
                ))),
                Self::WINDOWSIZE => options.push(TftpOption::WindowSize(slice_to_usize(
                    options_raw.next().unwrap(),
                ))),
                Self::END => return options,
                _ => unimplemented!("We dont handle option {option:X?}"),
            };
        }
        options
    }

    fn confirm(option: &Self) -> Self {
        match option {
            TftpOption::TransferSize(value) => TftpOption::TransferSize(*value),
            TftpOption::BlockSize(value) => TftpOption::BlockSize(*value),
            TftpOption::WindowSize(value) => TftpOption::WindowSize(*value),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum OpCode {
    ReadRequest = 1,
    Data = 3,
    Acknowledgement = 4,
    ErrorCode = 5,
    OptionAcknowledgement = 6,
}

impl TryFrom<u8> for OpCode {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::ReadRequest),
            3 => Ok(Self::Data),
            4 => Ok(Self::Acknowledgement),
            5 => Ok(Self::ErrorCode),
            6 => Ok(Self::OptionAcknowledgement),
            value => Err(Error::InvalidTftpOpCode(value)),
        }
    }
}

impl Serialise for OpCode {
    fn serialise(&self) -> Vec<u8> {
        let mut bytes = [0, 0];
        bytes[1] = *self as u8;
        bytes.to_vec()
    }
}

#[derive(Debug)]
pub enum Tftp<'tftp> {
    ReadRequest(ReadRequest<'tftp>),
    Acknowledgement(Acknowledgement),
    OptionAcknowledgement(OptionAcknowledgement),
    Data(Data<'tftp>),
}

#[derive(Debug)]
pub struct Data<'data> {
    block: u16,
    data: &'data [u8],
}

impl<'data> Data<'data> {
    fn new(ack: &Acknowledgement) -> Self {
        Self {
            block: ack.block,
            data: "I Love Naomi".as_bytes(),
        }
    }
}

#[derive(Debug)]
pub struct OptionAcknowledgement {
    options: Vec<TftpOption>,
}

impl OptionAcknowledgement {
    fn new(read_request: &ReadRequest, tsize: usize) -> Self {
        let options: Vec<TftpOption> = read_request
            .options
            .iter()
            .map(|option| match option {
                TftpOption::TransferSize(_) => TftpOption::TransferSize(tsize),
                option => TftpOption::confirm(option),
            })
            .collect();
        Self { options }
    }
}

#[derive(Debug)]
pub struct Acknowledgement {
    block: u16,
}

#[derive(Debug)]
pub struct ReadRequest<'tftp> {
    filename: &'tftp CStr,
    mode: &'tftp CStr,
    options: Vec<TftpOption>,
}

impl Serialise for Tftp<'_> {
    fn serialise(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        match self {
            Tftp::OptionAcknowledgement(res) => {
                bytes.extend_from_slice(&OpCode::OptionAcknowledgement.serialise());
                res.options
                    .iter()
                    .for_each(|option| bytes.extend_from_slice(&option.serialise()));
                bytes
            }
            Tftp::Data(res) => {
                bytes.extend_from_slice(&OpCode::Data.serialise());
                bytes.extend_from_slice(&res.block.to_be_bytes());
                bytes.extend_from_slice(&res.data);
                bytes
            }
            _ => panic!("{self:?}"),
        }
    }
}

impl<'tftp> Tftp<'tftp> {
    const OP_CODE_LEN: usize = 2;
    const NULL_SIZE: usize = 1;

    fn parse(data: &'tftp [u8]) -> Self {
        dbg!(data);
        let op_code = data[1].try_into();
        let mut ptr = Self::OP_CODE_LEN;

        match op_code {
            Ok(OpCode::ReadRequest) => {
                let filename = CStr::from_bytes_until_nul(&data[ptr..]).unwrap();
                ptr += filename.to_bytes().len() + Self::NULL_SIZE;

                let mode = CStr::from_bytes_until_nul(&data[ptr..]).unwrap();
                ptr += mode.to_bytes().len() + Self::NULL_SIZE;

                let options = TftpOption::parse(&data[ptr..]);

                Self::ReadRequest(ReadRequest {
                    filename,
                    mode,
                    options,
                })
            }
            Ok(OpCode::Acknowledgement) => Self::Acknowledgement(Acknowledgement {
                block: u16::from_be_bytes([data[ptr], data[ptr + 1]]),
            }),
            _ => panic!("{op_code:?}"),
        }
    }

    fn respond(&self, session: &mut Session) -> Self {
        match self {
            Self::ReadRequest(req) => {
                let mut file = File::open(req.filename.to_str().unwrap()).unwrap();
                let size = file.read_to_end(&mut session.data).unwrap();
                Tftp::OptionAcknowledgement(OptionAcknowledgement::new(req, size))
            }
            Self::Acknowledgement(req) => Tftp::Data(Data::new(req)),
            _ => panic!("{self:?}"),
        }
    }

    pub fn handle(session: &mut Session, data: &'tftp [u8]) -> Vec<u8> {
        Self::parse(data).respond(session).serialise()
    }
}
