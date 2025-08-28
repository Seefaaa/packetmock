#![cfg(windows)]

mod windivert;

use std::mem::zeroed;

use log::{LevelFilter, info};
use windivert_sys::WINDIVERT_ADDRESS;

use crate::windivert::WinDivert;

const FAKE_HTTP_REQUEST: &[u8] =
    b"GET / HTTP/1.1\r\nHost: www.w3.org\r\nUser-Agent: curl/8.14.1\r\nAccept: */*\r\nAccept-Encoding: deflate, gzip, br\r\n\r\n";
const TTL: u8 = 4;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();

    let windivert = WinDivert::open("outbound and tcp.DstPort == 80 and !impostor")?;

    let mut buffer = [0; 9016];
    let mut address: WINDIVERT_ADDRESS = unsafe { zeroed() };

    info!("Intercepting packets... Press Ctrl+C to stop.");

    while let Ok(packet) = windivert.recv(&mut buffer, &mut address) {
        if packet.data().is_some() {
            let mut packet_copy = packet.clone();

            packet_copy.set_data(FAKE_HTTP_REQUEST)?;
            packet_copy.ip_header_mut().TTL = TTL;

            windivert.send(packet_copy)?;
        }

        windivert.send(packet)?;
    }

    Ok(())
}
