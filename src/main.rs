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

/// Once a [UdpSocket] has been accept we check to see if we already have an open [Session] with
/// that socket, if we don't we create a branch new [Session] to store the state of our transaction
/// with a client. If we do have a [Session] we carry on where we left off.
///
/// We take the data from the socket and the client [Session] and pass to [Tftp::handle()]. If the
/// data from a client is valid and implemented by us we respond correctly, else we silently drop
/// the client from out list of [TftpSessions]
fn handle(socket: &UdpSocket, client: &SocketAddr, sessions: TftpSessions, data: &[u8]) {
    let mut sessions = sessions.lock().unwrap();
    let mut session = sessions.entry(*client).or_insert(Session::new());

    match Tftp::handle(&mut session, data) {
        Some(response) => _ = socket.send_to(&response, client).unwrap(),
        None => _ = sessions.remove(client).unwrap(),
    };
}
