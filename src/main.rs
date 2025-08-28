#![cfg(windows)]

mod http;
mod mock;
mod windivert;

use std::mem::zeroed;

use env_logger::Env;
use log::info;
use windivert_sys::WINDIVERT_ADDRESS;

use crate::{
    http::is_client_hello,
    mock::{FAKE_CLIENT_HELLO, FAKE_HTTP_REQUEST},
    windivert::{BUFFER_SIZE, WINDIVERT_FILTER, WinDivert},
};

const TTL: u8 = 4;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let windivert = WinDivert::open(WINDIVERT_FILTER)?;

    let mut buffer = [0; BUFFER_SIZE];
    let mut address: WINDIVERT_ADDRESS = unsafe { zeroed() };

    info!("Intercepting packets... Press Ctrl+C to stop.");

    while let Ok(packet) = windivert.recv(&mut buffer, &mut address) {
        match u16::from_be(packet.tcp_header().DstPort) {
            // HTTP
            80 => {
                if packet.data().is_some() {
                    let mut packet_copy = packet.try_clone()?;
                    packet_copy.set_data(FAKE_HTTP_REQUEST)?;
                    packet_copy.ip_header_mut().TTL = TTL;
                    windivert.send(packet_copy)?;
                }
            }
            // HTTPS
            443 => {
                if is_client_hello(&packet) {
                    let mut packet_copy = packet.try_clone()?;
                    packet_copy.set_data(FAKE_CLIENT_HELLO)?;
                    packet_copy.ip_header_mut().TTL = TTL;
                    windivert.send(packet_copy)?;
                }
            }
            _ => unreachable!(),
        }

        windivert.send(packet)?;
    }

    Ok(())
}
