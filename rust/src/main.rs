use std::{collections::HashMap, io::Read, net::{SocketAddr, TcpListener, TcpStream}, os::fd::{AsRawFd, RawFd}, time::Duration};
mod epoll;
use epoll::{Epoll, EventFlags};

struct ClientData {
    pub stream: TcpStream,
    pub addr: SocketAddr,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:51717")?;
    listener.set_nonblocking(true)?;

    println!("Running server on 51717");

    let mut epoll = Epoll::create()?;
    println!("Created epoll instance");
    epoll.add(&listener, EventFlags::EPOLLET | EventFlags:: EPOLLIN)?;
    println!("Added server fd to epoll instance");

    let mut clients: HashMap<RawFd, ClientData> = HashMap::new();

    loop {
        for event in epoll.wait(Duration::from_secs(60))? {
            if event.fd == listener.as_raw_fd() {
                let (stream, addr) = listener.accept()?;
                epoll.add(&stream, EventFlags::EPOLLIN | EventFlags::EPOLLET | EventFlags::EPOLLHUP | EventFlags::EPOLLRDHUP )?;
                clients.insert(stream.as_raw_fd(), ClientData { stream, addr });
                println!("Client connected {}:{}", addr.ip(), addr.port());
                continue;
            }

            if event.flags.contains(EventFlags::EPOLLIN) {
                let client = clients.get_mut(&event.fd).unwrap();
                let mut buffer = vec![0u8; 1024];
                client.stream.read(&mut buffer)?;
                println!("Client {}:{}: {}", client.addr.ip(), client.addr.port(), String::from_utf8_lossy(&buffer));
            }

            if event.flags.contains(EventFlags::EPOLLHUP) || event.flags.contains(EventFlags::EPOLLRDHUP) {
                println!("{}, {}", event.flags.contains(EventFlags::EPOLLHUP), event.flags.contains(EventFlags::EPOLLRDHUP));
                let client = clients.get_mut(&event.fd).unwrap();
                println!("Client disconnected {}:{}", client.addr.ip(), client.addr.port());
                epoll.delete(&event.fd)?;
                clients.remove(&event.fd);
            }
        };
    }

    Ok(())
}