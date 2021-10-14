use crate::{EncryptedDevice, OsInfo, Request};
use envfile::EnvFile;
use pop_disk_manager::os_probe::OsEntry;
use postage::mpsc::Sender;
use postage::prelude::*;
use zbus::SignalContext;

/// DBus frontend which accepts requests and passes them on to the background.
pub struct Frontend {
    pub env: Option<EnvFile>,
    pub sender: Sender<Request>,
}

#[dbus_interface(name = "com.system76.Distinst")]
impl Frontend {
    /// Request to decrypt a `device` using `key`, and assigning it to `name`.
    async fn decrypt(&mut self, device: String, key: String) -> zbus::fdo::Result<()> {
        eprintln!("decrypting {}", device);
        let _ = self.sender.send(Request::Decrypt { device, key }).await;
        Ok(())
    }

    #[dbus_interface(signal)]
    pub async fn decrypt_err(ctx: &SignalContext<'_>, why: String) -> zbus::Result<()>;

    #[dbus_interface(signal)]
    pub async fn decrypt_ok(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// Initiate a rescan of disk information.
    async fn disk_rescan(&mut self) -> zbus::fdo::Result<()> {
        eprintln!("disk rescan");
        let _ = self.sender.send(Request::DiskRescan).await;
        Ok(())
    }

    #[dbus_interface(signal)]
    pub async fn disk_rescan_complete(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// Initiate search for encrypted devices.
    async fn encrypted_devices(&mut self) -> zbus::fdo::Result<()> {
        eprintln!("fetching encrypted devices");
        let _ = self.sender.send(Request::EncryptedDevices).await;
        Ok(())
    }

    #[dbus_interface(signal)]
    pub async fn encrypted_devices_err(ctx: &SignalContext<'_>, why: String) -> zbus::Result<()>;

    #[dbus_interface(signal)]
    pub async fn encrypted_devices_ok(
        ctx: &SignalContext<'_>,
        devices: Vec<EncryptedDevice>,
    ) -> zbus::Result<()>;

    /// System is in an environment which was requested to refresh an existing OS.
    async fn is_refresh(&self) -> bool {
        let refresh = self
            .env
            .as_ref()
            .map_or(false, |env| env.get("MODE") == Some("refresh"));

        eprintln!("checking if refresh mode is enabled: {}", refresh);
        refresh
    }

    /// System is in an OEM first-time setup environment.
    async fn is_oem_mode(&self) -> bool {
        let oem = self
            .env
            .as_ref()
            .map_or(false, |env| env.get("OEM_MODE") == Some("1"));

        eprintln!("checking if OEM mode is enabled: {}", oem);
        oem
    }

    /// Initiate a search for OS boot entries.
    async fn os_entries(&mut self) -> zbus::fdo::Result<()> {
        eprintln!("fetching OS entries");
        let _ = self.sender.send(Request::OsEntries).await;
        Ok(())
    }

    #[dbus_interface(signal)]
    pub async fn os_entries_err(ctx: &SignalContext<'_>, why: String) -> zbus::Result<()>;

    #[dbus_interface(signal)]
    pub async fn os_entries_ok(ctx: &SignalContext<'_>, entries: Vec<OsEntry>) -> zbus::Result<()>;

    /// Initiate a search of operating systems.
    async fn os_search(&mut self) -> zbus::fdo::Result<()> {
        eprintln!("searching for operating systems");
        let _ = self.sender.send(Request::OsSearch).await;
        Ok(())
    }

    #[dbus_interface(signal)]
    pub async fn os_search_err(ctx: &SignalContext<'_>, why: String) -> zbus::Result<()>;

    #[dbus_interface(signal)]
    pub async fn os_search_ok(ctx: &SignalContext<'_>, entries: Vec<OsInfo>) -> zbus::Result<()>;
}
