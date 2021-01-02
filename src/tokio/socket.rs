// Copyright (C) 2020 - Will Glozer. All rights reserved.

use std::io::{Error, ErrorKind, IoSlice, IoSliceMut, Result};
use std::net::SocketAddr;
use std::task::{Context, Poll};
use futures_core::ready;
use tokio::future::poll_fn;
use tokio::io::PollEvented;
use tokio::net::ToSocketAddrs;
use crate::mio::{self, Ready};
use crate::opt::{Level, Name, Opt};
use super::split::split;
use super::{Domain, Type, Protocol, RawSend, RawRecv};

pub struct RawSocket {
    io: PollEvented<mio::RawSocket>,
}

impl RawSocket {
    pub fn new(domain: Domain, kind: Type, protocol: Option<Protocol>) -> Result<Self> {
        let sys = mio::RawSocket::new(domain, kind, protocol)?;
        let io  = PollEvented::new(sys)?;
        Ok(RawSocket { io })
    }

    pub async fn bind<A: ToSocketAddrs>(&self, addr: A) -> Result<()> {
        let addr = sockaddr(addr).await?;
        self.io.get_ref().bind(&addr)
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.io.get_ref().local_addr()
    }

    pub async fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        poll_fn(|cx| self.poll_recv_from(cx, buf)).await
    }

    pub async fn recv_msg(
        &self,
        data: &[IoSliceMut<'_>],
        ctrl: Option<&mut [u8]>
    ) -> Result<(usize, SocketAddr)> {
        let ctrl = ctrl.unwrap_or(&mut []);
        poll_fn(|cx| self.poll_recv_msg(cx, data, ctrl)).await
    }

    pub async fn send_to<A: ToSocketAddrs>(&mut self, buf: &[u8], addr: A) -> Result<usize> {
        let addr = sockaddr(addr).await?;
        poll_fn(|cx| self.poll_send_to(cx, buf, &addr)).await
    }

    pub async fn send_msg<A: ToSocketAddrs>(
        &mut self,
        addr: A,
        data: &[IoSlice<'_>],
        ctrl: Option<&[u8]>
    ) -> Result<usize> {
        let addr = sockaddr(addr).await?;
        let ctrl = ctrl.unwrap_or(&mut []);
        poll_fn(|cx| self.poll_send_msg(cx, &addr, data, ctrl)).await
    }

    pub fn get_sockopt<O: Opt>(&self, level: Level, name: Name) -> Result<O> {
        self.io.get_ref().get_sockopt(level, name)
    }

    pub fn set_sockopt<O: Opt>(&self, level: Level, name: Name, value: &O) -> Result<()> {
        self.io.get_ref().set_sockopt(level, name, value)
    }

    pub fn split(self) -> (RawRecv, RawSend) {
        split(self)
    }

    pub(crate) fn poll_recv_from(
        &self,
        cx:  &mut Context<'_>,
        buf: &mut [u8]
    ) -> Poll<Result<(usize, SocketAddr)>> {
        ready!(self.io.poll_read_ready(cx, Ready::readable()))?;

        match self.io.get_ref().recv_from(buf) {
            Err(ref e) if is_would_block(e) => self.clear_read_ready(cx)?,
            x                               => Poll::Ready(x),
        }
    }

    pub(crate) fn poll_recv_msg(
        &self,
        cx:   &mut Context<'_>,
        data: &[IoSliceMut<'_>],
        ctrl: &mut [u8],
    ) -> Poll<Result<(usize, SocketAddr)>> {
        ready!(self.io.poll_read_ready(cx, Ready::readable()))?;

        match self.io.get_ref().recv_msg(data, ctrl) {
            Err(ref e) if is_would_block(e) => self.clear_read_ready(cx)?,
            x                               => Poll::Ready(x),
        }
    }

    pub(crate) fn poll_send_to(
        &self,
        cx:   &mut Context<'_>,
        buf:  &[u8],
        addr: &SocketAddr
    ) -> Poll<Result<usize>> {
        ready!(self.io.poll_write_ready(cx))?;

        match self.io.get_ref().send_to(buf, addr) {
            Err(ref e) if is_would_block(e) => self.clear_write_ready(cx)?,
            x                               => Poll::Ready(x),
        }
    }

    pub(crate) fn poll_send_msg(
        &self,
        cx:   &mut Context<'_>,
        addr: &SocketAddr,
        data: &[IoSlice<'_>],
        ctrl: &[u8],
    ) -> Poll<Result<usize>> {
        ready!(self.io.poll_write_ready(cx))?;

        match self.io.get_ref().send_msg(addr, data, ctrl) {
            Err(ref e) if is_would_block(e) => self.clear_write_ready(cx)?,
            x                               => Poll::Ready(x),
        }
    }

    fn clear_read_ready<T>(&self, cx: &mut Context<'_>) -> Result<Poll<T>> {
        self.io.clear_read_ready(cx, Ready::readable())?;
        Ok(Poll::Pending)
    }

    fn clear_write_ready<T>(&self, cx: &mut Context<'_>) -> Result<Poll<T>> {
        self.io.clear_write_ready(cx)?;
        Ok(Poll::Pending)
    }
}

fn is_would_block(e: &Error) -> bool {
    e.kind() == ErrorKind::WouldBlock
}

async fn sockaddr<A: ToSocketAddrs>(addr: A) -> Result<SocketAddr> {
    match addr.to_socket_addrs().await?.next() {
        Some(addr) => Ok(addr),
        None       => Err(Error::new(ErrorKind::InvalidInput, "invalid socket address")),
    }
}
