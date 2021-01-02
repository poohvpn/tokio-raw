// Copyright (C) 2020 - Will Glozer. All rights reserved.

use std::io::{IoSlice, IoSliceMut, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::future::poll_fn;
use super::RawSocket;

pub struct RawRecv(Arc<RawSocket>);
pub struct RawSend(Arc<RawSocket>);

pub(crate) fn split(sock: RawSocket) -> (RawRecv, RawSend) {
    let recv = Arc::new(sock);
    let send = recv.clone();
    (RawRecv(recv), RawSend(send))
}

impl RawRecv {
    pub async fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        poll_fn(|cx| self.0.poll_recv_from(cx, buf)).await
    }

    pub async fn recv_msg(
        &self,
        data: &[IoSliceMut<'_>],
        ctrl: Option<&mut [u8]>
    ) -> Result<(usize, SocketAddr)> {
        let ctrl = ctrl.unwrap_or(&mut []);
        poll_fn(|cx| self.0.poll_recv_msg(cx, data, ctrl)).await
    }
}

impl RawSend {
    pub async fn send_to(&mut self, buf: &[u8], addr: &SocketAddr) -> Result<usize> {
        poll_fn(|cx| self.0.poll_send_to(cx, buf, addr)).await
    }

    pub async fn send_msg(
        &mut self,
        addr: &SocketAddr,
        data: &[IoSlice<'_>],
        ctrl: Option<&[u8]>
    ) -> Result<usize> {
        let ctrl = ctrl.unwrap_or(&[]);
        poll_fn(|cx| self.0.poll_send_msg(cx, addr, data, ctrl)).await
    }
}
