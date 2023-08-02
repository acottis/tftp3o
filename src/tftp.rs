use crate::error::Error;
use std::{ffi::CStr, str::from_utf8};

const NULL: u8 = 0x00;

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
        bytes.push(NULL);
        bytes.extend_from_slice(value.to_string().as_bytes());
        bytes.push(NULL);

        bytes
    }
}

impl TftpOption {
    const TSIZE: &[u8] = &[0x74, 0x73, 0x69, 0x7a, 0x65];
    const BLKSIZE: &[u8] = &[0x62, 0x6c, 0x6b, 0x73, 0x69, 0x7a, 0x65];
    const WINDOWSIZE: &[u8] = &[0x77, 0x69, 0x6e, 0x64, 0x6f, 0x77, 0x73, 0x69, 0x7a, 0x65];
    const END: &[u8] = &[];

    fn parse(data: &[u8]) -> Vec<TftpOption> {
        let mut options = Vec::with_capacity(5);
        let mut options_raw = data.split(|chr| *chr == NULL);

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
    ErrorCode = 5,
    OptionAcknowledgement = 6,
}

impl TryFrom<u8> for OpCode {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::ReadRequest),
            5 => Ok(Self::ErrorCode),
            6 => Ok(Self::OptionAcknowledgement),
            value => Err(Error::InvalidTftpOpCode(value)),
        }
    }
}

impl Serialise for OpCode {
    fn serialise(&self) -> Vec<u8> {
        let mut bytes = [0,0];
        bytes[1] = *self as u8;
        bytes.to_vec()
    }
}

#[derive(Debug)]
pub struct Tftp<'tftp> {
    op_code: OpCode,
    filename: Option<&'tftp CStr>,
    mode: Option<&'tftp CStr>,
    options: Vec<TftpOption>,
}

impl Serialise for Tftp<'_> {
    fn serialise(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.op_code.serialise());

        self.options
            .iter()
            .for_each(|option| bytes.extend_from_slice(&option.serialise()));

        bytes
    }
}

impl<'tftp> Tftp<'tftp> {
    const OP_CODE_LEN: usize = 2;
    const NULL_SIZE: usize = 1;

    fn parse(data: &'tftp [u8]) -> Self {
        dbg!(data);
        let op_code = data[1].try_into().unwrap();
        let mut ptr = Self::OP_CODE_LEN;

        let filename = CStr::from_bytes_until_nul(&data[ptr..]).unwrap();
        ptr += filename.to_bytes().len() + Self::NULL_SIZE;

        let mode = CStr::from_bytes_until_nul(&data[ptr..]).unwrap();
        ptr += mode.to_bytes().len() + Self::NULL_SIZE;

        let options = TftpOption::parse(&data[ptr..]);

        Self {
            op_code,
            filename: Some(filename),
            mode: Some(mode),
            options,
        }
    }

    fn acknowledge(&self) -> Self {

        let options: Vec<TftpOption> = self.options
            .iter()
            .filter(|option| {
                match option{
                    TftpOption::TransferSize(_) => false,
                    _=> true,
                }
            }) 
            .map(TftpOption::confirm)
            .collect();

        Self {
            op_code: OpCode::OptionAcknowledgement,
            filename: None,
            mode: None,
            options,
        }
    }

    pub fn handle(data: &'tftp [u8]) -> Vec<u8> {
        let tftp = &Self::parse(data);
        // dbg!(tftp);

        match tftp.op_code {
            OpCode::ReadRequest => tftp.acknowledge().serialise(),
            OpCode::ErrorCode => unimplemented!("ErrorCode {tftp:?}"),
            _ => todo!("Dont handle op_code yet"),
        }
    }
}
