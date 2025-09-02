use color_eyre::{Result, eyre::Context};
use windows_registry::LOCAL_MACHINE;

use crate::{
    REGISTRY_NAME,
    service::{ServiceState, query_service, start_service, stop_service},
};

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

pub fn set_ttl(ttl: u8) -> Result<()> {
    let key = LOCAL_MACHINE.create(format!("Software\\{REGISTRY_NAME}"))?;
    key.set_u32("TTL", ttl as u32)?;

    if let Ok(ServiceState::Running) = query_service() {
        stop_service().wrap_err("Failed to restart the service")?;
        start_service().wrap_err("Failed to restart the service")?;
    };

    Ok(())
}
