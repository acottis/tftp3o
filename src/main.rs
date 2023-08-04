use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    sync::{Arc, Mutex},
};

const PORT: u16 = 69;
const UDP_BUFFER_SIZE: usize = 512;
const BIND_ADDR: &str = "0.0.0.0";

mod error;
mod tftp;
use tftp::{Session, Tftp};

type TftpSessions = Arc<Mutex<HashMap<SocketAddr, Session>>>;

fn main() {
    let socket = UdpSocket::bind((BIND_ADDR, PORT)).unwrap();
    let sessions: TftpSessions = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let mut buffer = [0u8; UDP_BUFFER_SIZE];

        match socket.recv_from(&mut buffer) {
            Ok((len, client)) => handle(&socket, &client, sessions.clone(), &buffer[..len]),
            Err(_) => todo!(),
        }
    }
}

fn handle(socket: &UdpSocket, client: &SocketAddr, sessions: TftpSessions, data: &[u8]) {
    dbg!(socket);

    let mut sessions = sessions.lock().unwrap();
    let mut session = sessions.entry(*client).or_insert(Session::new());

    let response = Tftp::handle(&mut session, data);
    socket.send_to(&response, client).unwrap();
}
