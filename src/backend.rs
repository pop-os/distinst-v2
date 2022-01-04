// Copyright 2021 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Context;
use pop_disk_manager::os_probe;
use pop_disk_manager::{os_probe::OsEntry, ACellOwner, DiskManager, UDev};

use crate::frontend::Frontend;
use crate::{Device, EncryptedDevice, OsInfo, Request};
use std::future::Future;
use std::path::Path;
use zbus::{Connection, SignalContext};

/// DBus backend which carries out requests it receives.
pub struct Backend {
    pub disk_manager: DiskManager,
    pub t: ACellOwner,
}

impl Backend {
    /// Performs requests in the background between requests, emitting signals as necessary.
    pub async fn on_event(&mut self, conn: &Connection, event: Request) {
        self.with(conn, |backend, ctx| async move {
            match event {
                Request::Decrypt { device, key } => match dbg!(backend.decrypt(&device, &key)) {
                    Ok(()) => Frontend::decrypt_ok(&ctx).await,
                    Err(why) => Frontend::decrypt_err(&ctx, why.to_string()).await,
                },

                Request::DiskRescan => {
                    let _ = dbg!(backend.disk_rescan());
                    Frontend::disk_rescan_complete(&ctx).await
                }

                Request::EncryptedDevices => match dbg!(backend.encrypted_devices()) {
                    Ok(devices) => Frontend::encrypted_devices_ok(&ctx, devices).await,
                    Err(why) => Frontend::encrypted_devices_err(&ctx, why.to_string()).await,
                },

                Request::OsEntries => match dbg!(backend.os_entries()) {
                    Ok(entries) => Frontend::os_entries_ok(&ctx, entries).await,
                    Err(why) => Frontend::os_entries_err(&ctx, why.to_string()).await,
                },

                Request::OsSearch => match dbg!(backend.os_search()) {
                    Ok(entries) => Frontend::os_search_ok(&ctx, entries).await,
                    Err(why) => Frontend::os_search_err(&ctx, why.to_string()).await,
                },
            }
        })
        .await;
    }

    pub fn decrypt(&mut self, device: &str, key: &str) -> anyhow::Result<()> {
        let &mut Self {
            ref mut disk_manager,
            ref mut t,
            ..
        } = self;

        let udev = &mut udev_context()?;

        let mut name = String::new();

        loop {
            use rand::Rng;

            name.push_str("crypt-");
            name.extend(
                rand::thread_rng()
                    .sample_iter(&rand::distributions::Alphanumeric)
                    .take(6)
                    .map(char::from),
            );

            if Path::new("/dev/mapper/").join(&name).exists() {
                name.clear();
                continue;
            }

            break;
        }

        let devname: String = {
            let (devname, _) = disk_manager
                .block_by_uuid(device, &t)
                .context("could not find block device by UUID")?;

            devname.to_owned()
        };

        eprintln!("attempting to decrypt {}", devname);

        disk_manager
            .luks_unlock(&devname, &name, key.as_bytes(), udev, t)
            .context("err to unlock device")
    }

    pub fn disk_rescan(&mut self) -> anyhow::Result<()> {
        let &mut Self {
            ref mut disk_manager,
            ref mut t,
            ..
        } = self;

        let udev = &mut udev_context()?;
        disk_manager.reload(udev, t);
        Ok(())
    }

    pub fn encrypted_devices(&self) -> anyhow::Result<Vec<EncryptedDevice>> {
        let &Self {
            ref disk_manager,
            ref t,
            ..
        } = self;

        let mut encrypted = Vec::new();

        for (devname, block) in disk_manager.blocks.iter() {
            let device = DiskManager::device_from_block(block, t);
            if let Some(fs) = device.fs.as_ref() {
                if fs.type_ == "crypto_LUKS" && device.children.is_empty() {
                    encrypted.push(EncryptedDevice {
                        device: Device {
                            path: devname.to_owned(),
                        },
                        uuid: fs.uuid.clone(),
                    });
                }
            }
        }

        Ok(encrypted)
    }

    pub fn os_entries(&self) -> anyhow::Result<Vec<OsEntry>> {
        let &Self {
            ref disk_manager,
            ref t,
            ..
        } = self;

        Ok(os_probe::boot_entries(disk_manager, t))
    }

    pub fn os_search(&self) -> anyhow::Result<Vec<OsInfo>> {
        let &Self {
            ref disk_manager, ..
        } = self;

        let mut operating_systems = Vec::new();

        for devname in disk_manager.blocks.keys() {
            if let Some(linux) = os_probe::linux(&Path::new(&devname)) {
                operating_systems.push(OsInfo {
                    device: Device {
                        path: devname.clone(),
                    },
                    identifier: "linux",
                    name: linux.release.name.clone(),
                    version: linux.release.version.clone(),
                })
            }
        }

        Ok(operating_systems)
    }

    pub fn reload(&mut self) -> anyhow::Result<()> {
        let mut udev = udev_context()?;
        self.disk_manager.reload(&mut udev, &mut self.t);
        Ok(())
    }

    pub async fn with<'a, C, F>(&'a mut self, conn: &Connection, future: C)
    where
        C: FnOnce(&'a mut Self, SignalContext<'a>) -> F + 'a,
        F: Future<Output = zbus::Result<()>> + 'a,
    {
        if let Ok(iface) = conn
            .object_server()
            .interface::<_, Frontend>(crate::IFACE)
            .await
        {
            if let Err(why) = future(self, iface.signal_context().to_owned()).await {
                eprintln!("dbus backend context error: {:?}", why);
            }
        }
    }
}

fn udev_context() -> anyhow::Result<UDev> {
    let context = libudev::Context::new().context("could not get libudev context")?;

    let mut enumerator =
        libudev::Enumerator::new(&context).context("could not get libudev enumerator")?;

    enumerator
        .match_subsystem("block")
        .context("err to match block subsystem for libudev enumerator")?;

    Ok(UDev {
        context,
        enumerator,
    })
}
