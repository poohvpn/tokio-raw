// Copyright (C) 2020 - Will Glozer. All rights reserved.

pub use socket::RawSocket;

pub use socket2::Domain;
pub use socket2::Type;
pub use socket2::Protocol;
pub use mio::Ready;

mod socket;

#[cfg(test)]
mod test {
    use std::io::{Error, ErrorKind, IoSlice, IoSliceMut, Result};
    use std::net::{IpAddr, SocketAddr};
    use std::thread::sleep;
    use std::time::Duration;
    use libc::{c_int, SOCK_DGRAM, SOCK_STREAM};
    use crate::{Domain, Type, Level, Name};
    use super::RawSocket;

    #[test]
    fn get_sockopt() -> Result<()> {
        let ipv4  = Domain::ipv4();

        let sock0 = RawSocket::new(ipv4, Type::dgram(),  None)?;
        let sock1 = RawSocket::new(ipv4, Type::stream(), None)?;

        assert_eq!(SOCK_DGRAM,  sock0.get_sockopt(Level::SOCKET, Name::SO_TYPE)?);
        assert_eq!(SOCK_STREAM, sock1.get_sockopt(Level::SOCKET, Name::SO_TYPE)?);

        Ok(())
    }

    #[test]
    fn set_sockopt() -> Result<()> {
        let sock = RawSocket::new(Domain::ipv4(), Type::stream(), None)?;

        let set: c_int = 1;
        sock.set_sockopt(Level::SOCKET, Name::SO_KEEPALIVE, &set)?;

        assert_eq!(set, sock.get_sockopt(Level::SOCKET, Name::SO_KEEPALIVE)?);

        Ok(())
    }

    #[test]
    fn send_recv_msg() -> Result<()> {
        let addr = SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 0);

        let send = RawSocket::new(Domain::ipv4(), Type::dgram(), None)?;
        let recv = RawSocket::new(Domain::ipv4(), Type::dgram(), None)?;

        send.bind(&addr)?;
        recv.bind(&addr)?;

        let sent = [0u8; 64];
        let addr = recv.local_addr()?;
        let n = send.send_msg(&addr, &[IoSlice::new(&sent)], &[])?;

        assert_eq!(n, sent.len());

        let mut data = [0u8; 64];

        let (n, from) = loop {
            let iovec = &[IoSliceMut::new(&mut data)];
            let delay = Duration::from_secs(1);

            let is_wb = |e: &Error| e.kind() == ErrorKind::WouldBlock;

            match recv.recv_msg(iovec, &mut []) {
                Ok((n, from))          => break (n, from),
                Err(ref e) if is_wb(e) => sleep(delay),
                Err(e)                 => return Err(e),
            }
        };

        assert_eq!(n, data.len());
        assert_eq!(&sent[..], &data[..]);
        assert_eq!(from, send.local_addr()?);

        Ok(())
    }
}
