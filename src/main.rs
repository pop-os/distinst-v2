// Copyright 2021 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

#[macro_use]
extern crate zbus;

pub mod backend;
pub mod frontend;

use crate::backend::Backend;
use crate::frontend::Frontend;
use anyhow::Context;
use pop_disk_manager::{ACellOwner, DiskManager};
use postage::mpsc;
use postage::prelude::*;
use std::path::Path;
use zbus::ConnectionBuilder;

const IFACE: &str = "/com/system76/Distinst";

fn main() -> anyhow::Result<()> {
    better_panic::install();

    async_io::block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    let dm = devicemapper::DM::new()
        .map_err(|why| anyhow::anyhow!("{}", why))
        .context("failed to initialize devicemapper instanace")?;

    let (sender, mut receiver) = mpsc::channel(2);

    let mut backend = Backend {
        disk_manager: DiskManager::new(dm),
        t: ACellOwner::new(),
    };

    if let Err(why) = backend.reload() {
        eprintln!("failed to reload disk manager: {}", why);
    }

    let frontend = Frontend {
        env: envfile::EnvFile::new(&Path::new("/cdrom/recovery.conf")).ok(),
        sender,
    };

    eprintln!("initiating connection to system");

    let connection = ConnectionBuilder::system()
        .expect("failed to create system connection builder")
        .name("com.system76.Distinst")
        .expect("failed to set name for system service")
        .serve_at(IFACE, frontend)
        .expect("failed to serve interface")
        .build()
        .await
        .expect("failed to initialize dbus connection");

    eprintln!("initiated connection");

    let conn = connection.clone();

    // Processes all requests from the DBus frontend.
    let backend_event_loop = async move {
        while let Some(event) = receiver.recv().await {
            backend.on_event(&conn, event).await;
        }
    };

    backend_event_loop.await;

    Ok(())
}

use serde::{Deserialize, Serialize};
use zvariant::derive::Type;

#[derive(Debug, Type, Serialize, Deserialize)]
pub struct Device {
    pub path: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
pub struct EncryptedDevice {
    pub device: Device,
    pub uuid: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
pub struct OsInfo {
    pub device: Device,
    pub identifier: &'static str,
    pub name: String,
    pub version: String,
}

#[derive(Debug)]
pub enum Request {
    Decrypt { device: String, key: String },
    DiskRescan,
    EncryptedDevices,
    OsEntries,
    OsSearch,
}
