use std::{borrow::Cow, cmp::Ordering, ffi::CString, mem::zeroed, ptr::null_mut, slice};

use color_eyre::eyre::{ContextCompat, bail};
use const_format::formatcp as const_format;
use log::info;
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use windivert_sys::{
    WINDIVERT_ADDRESS, WINDIVERT_IPHDR, WINDIVERT_LAYER_WINDIVERT_LAYER_NETWORK, WINDIVERT_TCPHDR,
    WinDivertHelperCalcChecksums, WinDivertHelperParsePacket, WinDivertOpen, WinDivertRecv,
    WinDivertSend,
};

use crate::{
    TTL,
    http::is_client_hello,
    mock::{FAKE_CLIENT_HELLO, FAKE_HTTP_REQUEST},
};

pub const WINDIVERT_FILTER: &str = const_format!(
    "outbound and (tcp.DstPort == 80 or tcp.DstPort == 443) and tcp.PayloadLength > 0 and tcp.PayloadLength < {BUFFER_SIZE} and !impostor and !loopback and {LAN_FILTER}"
);
const LAN_FILTER: &str = "(
    (ip.DstAddr < 127.0.0.1 or ip.DstAddr > 127.255.255.255) and
    (ip.DstAddr < 10.0.0.0 or ip.DstAddr > 10.255.255.255) and
    (ip.DstAddr < 192.168.0.0 or ip.DstAddr > 192.168.255.255) and
    (ip.DstAddr < 172.16.0.0 or ip.DstAddr > 172.31.255.255) and
    (ip.DstAddr < 169.254.0.0 or ip.DstAddr > 169.254.255.255)
)";
pub const BUFFER_SIZE: usize = 9016;

/// A safe wrapper around a WinDivert handle and associated methods.
pub struct WinDivert {
    handle: windivert_sys::HANDLE,
}

impl WinDivert {
    /// Open a WinDivert handle with the specified filter.
    ///
    /// The filter syntax is documented at
    /// <https://reqrypt.org/windivert-doc.html#filter_language>.
    pub fn open(filter: &str) -> color_eyre::Result<Self> {
        let filter_cstr = CString::new(filter)?;

        let handle = unsafe {
            WinDivertOpen(
                filter_cstr.as_ptr(),
                WINDIVERT_LAYER_WINDIVERT_LAYER_NETWORK,
                0,
                0,
            )
        };

        if handle == INVALID_HANDLE_VALUE as _ {
            let err_code = unsafe { windivert_sys::GetLastError() };
            bail!("Failed to open WinDivert handle: {err_code}");
        }

        Ok(Self { handle })
    }

    /// Receive a packet from the WinDivert handle.
    ///
    /// The buffer must be large enough to hold the entire packet.
    pub fn recv<'a, 'b: 'a>(
        &self,
        buffer: &'b mut [u8],
        addr: &'b mut WINDIVERT_ADDRESS,
    ) -> color_eyre::Result<Packet<'a>> {
        let mut recv_len = 0;

        let result = unsafe {
            WinDivertRecv(
                self.handle,
                buffer.as_mut_ptr() as _,
                buffer.len() as _,
                &mut recv_len,
                addr,
            )
        };

        if result == 0 {
            let err_code = unsafe { windivert_sys::GetLastError() };
            bail!("Failed to receive packet: {err_code}");
        }

        let raw = &buffer[..recv_len as usize];

        Packet::new(Cow::Borrowed(raw), Cow::Borrowed(addr))
    }

    /// Send a packet to the WinDivert handle.
    pub fn send(&self, mut packet: Packet<'_>) -> color_eyre::Result<()> {
        if packet.recalc_checksums {
            packet.calc_checksums()?;
        }

        let result = unsafe {
            WinDivertSend(
                self.handle,
                packet.raw.as_ptr() as _,
                packet.raw.len() as _,
                null_mut(),
                packet.addr.as_ref(),
            )
        };

        if result == 0 {
            let err_code = unsafe { windivert_sys::GetLastError() };
            bail!("Failed to send packet: {err_code}");
        }

        Ok(())
    }
}

/// A representation of a network packet intercepted by WinDivert.
pub struct Packet<'a> {
    pub raw: Cow<'a, [u8]>,
    pub addr: Cow<'a, WINDIVERT_ADDRESS>,

    ip_header_ptr: *mut WINDIVERT_IPHDR,
    tcp_header_ptr: *mut WINDIVERT_TCPHDR,
    data_ptr: *mut u8,
    data_length: usize,

    recalc_checksums: bool,
}

impl<'a> Packet<'a> {
    /// Create a new `Packet` from raw packet data and address.
    pub fn new<'b: 'a>(
        raw: Cow<'b, [u8]>,
        addr: Cow<'b, WINDIVERT_ADDRESS>,
    ) -> color_eyre::Result<Self> {
        let mut ip_header = null_mut();
        let mut tcp_header = null_mut();
        let mut data = null_mut();
        let mut length = 0;

        let result = unsafe {
            WinDivertHelperParsePacket(
                raw.as_ptr() as _,
                raw.len() as _,
                &mut ip_header,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                &mut tcp_header,
                null_mut(),
                &mut data,
                &mut length,
                null_mut(),
                null_mut(),
            )
        };

        if result == 0 {
            let err_code = unsafe { windivert_sys::GetLastError() };
            bail!("Failed to parse packet: {err_code}");
        }

        Ok(Self {
            raw,
            addr,
            ip_header_ptr: ip_header,
            tcp_header_ptr: tcp_header,
            data_ptr: data as _,
            data_length: length as _,
            recalc_checksums: false,
        })
    }

    /// Re-parse the raw packet data to update internal pointers.
    /// This is necessary if the raw data has been modified or reallocated.
    fn reparse(&mut self) -> color_eyre::Result<()> {
        let mut ip_header = null_mut();
        let mut tcp_header = null_mut();
        let mut data = null_mut();
        let mut length = 0;

        let result = unsafe {
            WinDivertHelperParsePacket(
                self.raw.as_ptr() as _,
                self.raw.len() as _,
                &mut ip_header,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                &mut tcp_header,
                null_mut(),
                &mut data,
                &mut length,
                null_mut(),
                null_mut(),
            )
        };

        if result == 0 {
            let err_code = unsafe { windivert_sys::GetLastError() };
            bail!("Failed to parse packet: {err_code}");
        }

        self.ip_header_ptr = ip_header;
        self.tcp_header_ptr = tcp_header;
        self.data_ptr = data as _;
        self.data_length = length as _;

        Ok(())
    }

    /// Get a reference to the packet data without checking for null pointers or zero length.
    ///
    /// # Safety
    /// This function is unsafe because it does not check if the data pointer is null or if
    /// the data length is zero. The caller must ensure that these conditions are met before
    /// calling this function.
    #[inline]
    pub fn data_unchecked(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.data_ptr, self.data_length) }
    }

    /// Get a reference to the packet data, returning `None` if the data pointer is null or
    /// the data length is zero.
    #[inline]
    pub fn data(&self) -> Option<&[u8]> {
        if !self.data_ptr.is_null() && self.data_length > 0 {
            Some(self.data_unchecked())
        } else {
            None
        }
    }

    /// Get a mutable reference to the packet data without checking for null pointers or zero length.
    ///
    /// # Safety
    /// This function is unsafe because it does not check if the data pointer is null or if
    /// the data length is zero. The caller must ensure that these conditions are met before
    /// calling this function.
    #[inline]
    pub fn data_mut_unchecked(&mut self) -> &mut [u8] {
        self.recalc_checksums = true;
        unsafe { slice::from_raw_parts_mut(self.data_ptr, self.data_length) }
    }

    /// Get a mutable reference to the IP header.
    ///
    /// # Safety
    /// This function is unsafe because it does not check if the IP header pointer is null. The
    /// caller must ensure that this condition is met before calling this function.
    #[inline]
    pub fn ip_header_mut(&mut self) -> &mut WINDIVERT_IPHDR {
        self.recalc_checksums = true;
        unsafe { &mut *self.ip_header_ptr }
    }

    /// Get a reference to the TCP header.
    ////
    /// # Safety
    /// This function is unsafe because it does not check if the TCP header pointer is null. The
    /// caller must ensure that this condition is met before calling this function.
    #[inline]
    pub fn tcp_header(&self) -> &WINDIVERT_TCPHDR {
        unsafe { &*self.tcp_header_ptr }
    }

    /// Recalculate the checksums for the packet.
    /// This should be called after modifying the packet data or headers.
    fn calc_checksums(&mut self) -> color_eyre::Result<()> {
        let result = unsafe {
            WinDivertHelperCalcChecksums(
                self.raw.to_mut().as_mut_ptr() as _,
                self.raw.len() as _,
                &mut *self.addr.to_mut(),
                0,
            )
        };

        if result == 0 {
            let err_code = unsafe { windivert_sys::GetLastError() };
            bail!("Failed to calculate checksums: {err_code}");
        }

        Ok(())
    }

    /// Set the packet data, resizing the raw packet if necessary.
    /// This will also update the IP header length field accordingly.
    pub fn set_data(&mut self, data: &[u8]) -> color_eyre::Result<()> {
        let ordering = self
            .data()
            .map(|packet_data| packet_data.len().cmp(&data.len()))
            .context("Packet has no data")?; // no need to handle this case for now

        match ordering {
            Ordering::Less | Ordering::Greater => {
                let diff = data.len() as isize - self.data_length as isize;

                let ip_header = self.ip_header_mut();
                let length = u16::from_be(ip_header.Length);
                ip_header.Length = (length as i16 + diff as i16).to_be() as u16;

                let length = (self.raw.len() as isize + diff) as usize;

                match self.raw {
                    Cow::Borrowed(raw) => {
                        let mut vec = Vec::with_capacity(length);
                        vec.extend_from_slice(raw);
                        vec.resize(length, 0);
                        self.raw = Cow::Owned(vec);
                    }
                    Cow::Owned(ref mut raw) => {
                        raw.resize(length, 0);
                    }
                }

                // raw slice may have been reallocated, so we need to reparse to get updated pointers
                self.reparse()?;
            }
            _ => {}
        }

        self.data_mut_unchecked().copy_from_slice(data);

        Ok(())
    }

    /// Create a deep copy of the packet, allocating new memory for the raw data and address.
    pub fn try_clone(&self) -> color_eyre::Result<Self> {
        let raw = Cow::Owned(self.raw.clone().into_owned());
        let addr = Cow::Owned(self.addr.clone().into_owned());

        Packet::new(raw, addr)
    }
}

/// Start intercepting packets and modifying them as necessary.
pub fn intercept() -> color_eyre::Result<()> {
    let windivert = WinDivert::open(WINDIVERT_FILTER)?;

    let mut buffer = [0; BUFFER_SIZE];
    let mut address: WINDIVERT_ADDRESS = unsafe { zeroed() };

    info!("Intercepting packets");

    loop {
        match windivert.recv(&mut buffer, &mut address) {
            Ok(packet) => {
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
            Err(e) => {
                bail!("Failed to receive packet: {e:?}");
            }
        }
    }
}
