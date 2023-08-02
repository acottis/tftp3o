use std::{net::{UdpSocket, SocketAddr}, thread};

const PORT: u16 = 69;
const UDP_BUFFER_SIZE: usize = 512;
const BIND_ADDR: &str = "0.0.0.0";

mod error;
mod tftp;
use tftp::Tftp;

fn main() {
    let socket = UdpSocket::bind((BIND_ADDR, PORT)).unwrap();

    loop {
        let mut buffer = [0u8; UDP_BUFFER_SIZE];

        match socket.recv_from(&mut buffer) {
            Ok((len, client)) => handle(&socket, &client, &buffer[..len]),
            Err(_) => todo!(),
        }
    }
}

fn handle(socket: &UdpSocket, client: &SocketAddr, data: &[u8]) {
    dbg!(socket);
    let response = Tftp::handle(data);
    socket.send_to(&response, client).unwrap();
}
