#![allow(dead_code)]

use std::collections::HashMap;

use color_eyre::eyre::bail;
use memchr::memmem::Finder;

use crate::windivert::Packet;

pub struct HttpHeaders {
    raw: HashMap<String, String>,
}

impl HttpHeaders {
    pub fn get(&self, name: &str) -> Option<&String> {
        self.raw.get(name)
    }
}

impl TryFrom<&[u8]> for HttpHeaders {
    type Error = color_eyre::Report;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut headers = HashMap::new();

        for line in str::from_utf8(value)?.lines().skip(1) {
            let Some((name, value)) = line.split_once(':') else {
                break;
            };

            headers.insert(
                name.to_lowercase().trim().to_string(),
                value.trim().to_string(),
            );
        }

        if !headers.is_empty() {
            return Ok(HttpHeaders { raw: headers });
        }

        bail!("No HTTP headers found");
    }
}

fn extract_sni<'a>(packet: &'a Packet<'_>) -> Option<&'a str> {
    let data = packet.data()?;
    let finder = Finder::new(&[0; 3]);

    // Look for the SNI extension in the ClientHello packet
    // 0x00, 0x00, 0x00, <len1>, 0x00, <len2>, 0x00, 0x00, <len3>, <hostname>

    for pos in finder.find_iter(data) {
        match &data[pos..pos + 9] {
            // a: length of the SNI extension (2 bytes)
            // b: length of the Server Name list (2 bytes)
            // c: length of the Server Name (2 bytes)
            [0, 0, 0, a, 0, b, 0, 0, c] if *a == b + 2 && *b == c + 3 => {
                let start = pos + 9;
                let length = *c as usize;

                // unsafe
                if let Ok(host_name) = str::from_utf8(&data[start..start + length]) {
                    return Some(host_name);
                }
            }
            _ => {}
        }
    }

    None
}

/// Check if the given packet is a TLS ClientHello message
/// This is a very naive check and may not cover all cases
/// but works for most common scenarios
/// It checks if the first byte is 0x16 (Handshake)
/// and the sixth byte is 0x01 (ClientHello)
///
/// # Safety
/// This function assumes that the packet has a valid TCP payload
pub fn is_client_hello(packet: &Packet<'_>) -> bool {
    let data = packet.data_unchecked();

    data.first().map(|&b| b == 0x16).unwrap_or(false) // Handshake
        && data.get(5).map(|&b| b == 0x01).unwrap_or(false) // ClientHello
}
