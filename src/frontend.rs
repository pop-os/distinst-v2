// Copyright 2021 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{EncryptedDevice, OsInfo, Request};
use envfile::EnvFile;
use pop_disk_manager::os_probe::OsEntry;
use postage::mpsc::Sender;
use postage::prelude::*;
use std::collections::BTreeMap;
use zbus::SignalContext;

/// DBus frontend which accepts requests and passes them on to the background.
pub struct Frontend {
    pub env: Option<EnvFile>,
    pub sender: Sender<Request>,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Mode {
    Live = 0,
    Oem = 1,
    Recovery = 2,
    Refresh = 3,
}

#[dbus_interface(name = "com.system76.Distinst")]
impl Frontend {
    /// Request to decrypt a `device` using `key`, and assigning it to `name`.
    async fn decrypt(&mut self, device: String, key: String) -> zbus::fdo::Result<()> {
        eprintln!("decrypting {}", device);
        let _ = self.sender.send(Request::Decrypt { device, key }).await;
        Ok(())
    }

    #[rustfmt::skip]
    #[dbus_interface(signal)]
    pub async fn decrypt_err(ctx: &SignalContext<'_>, why: String) -> zbus::Result<()>;

    #[rustfmt::skip]
    #[dbus_interface(signal)]
    pub async fn decrypt_ok(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// Initiate a rescan of disk information.
    async fn disk_rescan(&mut self) -> zbus::fdo::Result<()> {
        eprintln!("disk rescan");
        let _ = self.sender.send(Request::DiskRescan).await;
        Ok(())
    }

    #[rustfmt::skip]
    #[dbus_interface(signal)]
    pub async fn disk_rescan_complete(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// Initiate search for encrypted devices.
    async fn encrypted_devices(&mut self) -> zbus::fdo::Result<()> {
        eprintln!("fetching encrypted devices");
        let _ = self.sender.send(Request::EncryptedDevices).await;
        Ok(())
    }

    #[rustfmt::skip]
    #[dbus_interface(signal)]
    pub async fn encrypted_devices_err(ctx: &SignalContext<'_>, why: String) -> zbus::Result<()>;

    #[rustfmt::skip]
    #[dbus_interface(signal)]
    pub async fn encrypted_devices_ok(
        ctx: &SignalContext<'_>,
        devices: Vec<EncryptedDevice>,
    ) -> zbus::Result<()>;

    /// Determines which mode the system is currently in.
    async fn mode(&self) -> u8 {
        let mode = if let Some(env) = self.env.as_ref() {
            if env.get("OEM_MODE") == Some("1") {
                Mode::Oem
            } else if env.get("MODE") == Some("refresh") {
                Mode::Refresh
            } else {
                Mode::Recovery
            }
        } else {
            Mode::Live
        };

        mode as u8
    }

    /// Obtain the recovery partition's configuration as a map.
    async fn recovery_config(&self) -> zbus::fdo::Result<BTreeMap<String, String>> {
        match self.env.as_ref().map(|env| env.store.clone()) {
            Some(map) => Ok(map),
            None => {
                zbus::fdo::Result::Err(zbus::fdo::Error::Failed("no recovery config found".into()))
            }
        }
    }

    /// Initiate a search for OS boot entries.
    async fn os_entries(&mut self) -> zbus::fdo::Result<()> {
        eprintln!("fetching OS entries");
        let _ = self.sender.send(Request::OsEntries).await;
        Ok(())
    }

    #[rustfmt::skip]
    #[dbus_interface(signal)]
    pub async fn os_entries_err(ctx: &SignalContext<'_>, why: String) -> zbus::Result<()>;

    #[rustfmt::skip]
    #[dbus_interface(signal)]
    pub async fn os_entries_ok(ctx: &SignalContext<'_>, entries: Vec<OsEntry>) -> zbus::Result<()>;

    /// Initiate a search of operating systems.
    async fn os_search(&mut self) -> zbus::fdo::Result<()> {
        eprintln!("searching for operating systems");
        let _ = self.sender.send(Request::OsSearch).await;
        Ok(())
    }

    #[rustfmt::skip]
    #[dbus_interface(signal)]
    pub async fn os_search_err(ctx: &SignalContext<'_>, why: String) -> zbus::Result<()>;

    #[rustfmt::skip]
    #[dbus_interface(signal)]
    pub async fn os_search_ok(ctx: &SignalContext<'_>, entries: Vec<OsInfo>) -> zbus::Result<()>;
}
