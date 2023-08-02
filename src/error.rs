#[derive(Debug)]
pub enum Error {
    InvalidTftpOpCode(u8),
}
