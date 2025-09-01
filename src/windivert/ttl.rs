use color_eyre::Result;
use windows_registry::LOCAL_MACHINE;

use crate::REGISTRY_NAME;

const DEFAULT_TTL: u8 = 4;

pub fn get_ttl() -> u8 {
    let Ok(key) = LOCAL_MACHINE.open(format!("Software\\{REGISTRY_NAME}")) else {
        return DEFAULT_TTL;
    };

    match key.get_u32("TTL") {
        Ok(ttl) => ttl as u8,
        Err(_) => DEFAULT_TTL,
    }
}

#[allow(dead_code)]
pub fn set_ttl(ttl: u8) -> Result<()> {
    let key = LOCAL_MACHINE.create(format!("Software\\{REGISTRY_NAME}"))?;
    Ok(key.set_u32("TTL", ttl as u32)?)
}
