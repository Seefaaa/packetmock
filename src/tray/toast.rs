use std::{
    env::temp_dir,
    fs::{create_dir_all, write},
};

use color_eyre::Result;
use windows::{
    Foundation::{IReference, PropertyValue},
    Globalization::Calendar,
    UI::Notifications::{ToastNotification, ToastNotificationManager, ToastTemplateType},
    core::Interface,
};
use windows_registry::LOCAL_MACHINE;

/// Display name for the toast notification.
const TOAST_DISPLAY_NAME: &str = "Packetmock";
/// Application ID for the toast notification.
const TOAST_APPID: &str = "Seefaaa.Packetmock";
/// Icon of the toast notification.
const TOAST_ICON: &[u8] = include_bytes!("../../resources/pink48.png");
// How long the toast should remain in the action center.
const TOAST_DURATION: i32 = 10;

/// Show a Windows toast notification with the specified message.
pub fn show_toast(message: &str) -> Result<()> {
    let temp = temp_dir().join(TOAST_APPID);
    let icon_path = temp.join("toast.png");

    create_dir_all(&temp)?;
    write(&icon_path, TOAST_ICON)?;

    let key = LOCAL_MACHINE
        .options()
        .volatile()
        .read()
        .write()
        .create()
        .open(format!("Software\\Classes\\AppUserModelId\\{TOAST_APPID}"))?;

    key.set_string("DisplayName", TOAST_DISPLAY_NAME)?;
    key.set_string("IconUri", icon_path.to_string_lossy().as_ref())?;

    let toast_template = ToastTemplateType::ToastImageAndText01;
    let toast_xml = ToastNotificationManager::GetTemplateContent(toast_template)?;

    let text_elements = toast_xml.GetElementsByTagName(&"text".into())?;

    if text_elements.Length()? > 0 {
        let message_element = text_elements.Item(0)?;
        message_element.SetInnerText(&message.into())?;
    }

    let toast = ToastNotification::CreateToastNotification(&toast_xml)?;

    let now = Calendar::new()?;
    now.AddSeconds(TOAST_DURATION)?;
    let dt = now.GetDateTime()?;
    toast.SetExpirationTime(&PropertyValue::CreateDateTime(dt)?.cast::<IReference<_>>()?)?;

    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&TOAST_APPID.into())?;
    notifier.Show(&toast)?;

    Ok(())
}
