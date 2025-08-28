use crate::windivert::Packet;

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
