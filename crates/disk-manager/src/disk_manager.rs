// Copyright 2021 System76 <info@system76.com>
// SPDX-License-Identifier: LGPL-3.0-only

use crate::block_types::*;
use crate::udev::UDev;
use crate::{ACell, ACellOwner};
use libcryptsetup_rs::LibcryptErr;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("cannot unlock device which does not exist")]
    DeviceNotFound,
    #[error("decryption failed")]
    Cryptsetup(#[source] LibcryptErr),
}

pub type DevName<'a> = &'a str;

pub struct DiskManager {
    pub dm: devicemapper::DM,
    pub blocks: BTreeMap<String, BlockDevice>,
}

impl DiskManager {
    pub fn new(dm: devicemapper::DM) -> Self {
        Self {
            dm,
            blocks: BTreeMap::new(),
        }
    }

    /// Locate a block by its `DEVNAME`.
    pub fn block_by_devname(&self, name: &str, _t: &ACellOwner) -> Option<BlockDevice> {
        self.blocks.get(name).cloned()
    }

    /// Locate a block device by PartUUID
    pub fn block_by_part_uuid<'a>(
        &'a self,
        uuid: &str,
        t: &ACellOwner,
    ) -> Option<(DevName<'a>, BlockDevice)> {
        for (devname, block) in self.blocks.iter() {
            if let BlockDevice::Partition(part) = block {
                if part.ro(t).uuid == uuid {
                    return Some((devname, block.clone()));
                }
            }
        }

        None
    }

    /// Locate a block device by FS UUID
    pub fn block_by_uuid<'a>(
        &'a self,
        uuid: &str,
        t: &ACellOwner,
    ) -> Option<(DevName<'a>, BlockDevice)> {
        for (devname, block) in self.blocks.iter() {
            if let Some(fs) = Self::device_from_block(block, t).fs.as_ref() {
                if fs.uuid == uuid {
                    return Some((devname, block.clone()));
                }
            }
        }

        None
    }

    /// Every block device has a `device` field.
    pub fn device_from_block<'a>(dev: &'a BlockDevice, t: &'a ACellOwner) -> &'a Device {
        match dev {
            BlockDevice::Disk(disk) => &disk.ro(t).device,
            BlockDevice::Partition(entry) => &entry.ro(t).device,
            BlockDevice::DeviceMap(dm) => &dm.ro(t).device,
        }
    }

    /// Locate a device map by `DM_NAME`.
    pub fn dm_by_dm_name(&self, dm_name: &str, t: &ACellOwner) -> Option<Arc<ACell<DeviceMap>>> {
        for block in self.blocks.values() {
            if let BlockDevice::DeviceMap(dm) = block {
                if dm.ro(t).name == dm_name {
                    return Some(dm.clone());
                }
            }
        }

        None
    }

    /// Locate a device map by its LV name.
    pub fn dm_by_lv_name(&self, lv_name: &str, t: &ACellOwner) -> Option<Arc<ACell<DeviceMap>>> {
        for block in self.blocks.values() {
            if let BlockDevice::DeviceMap(dm) = block {
                if let Some(lv) = dm.ro(t).lv_name.as_ref() {
                    if lv == lv_name {
                        return Some(dm.clone());
                    }
                }
            }
        }

        None
    }

    /// Reload block device information from the system.
    pub fn reload(&mut self, udev: &mut UDev, t: &mut ACellOwner) {
        self.blocks.clear();

        let devices = match udev.enumerator.scan_devices() {
            Ok(devices) => devices.collect::<Vec<_>>(),
            Err(_) => return,
        };

        for device in devices {
            udev.append(self, &device, t);
        }
    }

    /// Close a LUKS partition with libcryptsetup, deactivating its volumes.
    pub fn luks_lock(
        &mut self,
        device: &str,
        name: &str,
        udev: &mut UDev,
        t: &mut ACellOwner,
    ) -> Result<(), EncryptionError> {
        // Check if the device to be locked exists.
        let dev = self
            .blocks
            .get(device)
            .ok_or(EncryptionError::DeviceNotFound)?;

        // Get the children of this device that need to be deactivated.
        let dev = Self::device_from_block(dev, t);

        // Determine which VGs need to be deactivated, and deactivate them;
        {
            let mut children = dev.children.clone();

            let mut vgs_to_suspend = BTreeSet::new();
            let mut luks_to_lock = vec![device.to_owned()];

            while let Some(child_) = children.pop() {
                let child = child_.ro(t);

                if let Some(vg) = child.vg_name.clone() {
                    vgs_to_suspend.insert(vg);
                }

                if let Some(fs) = child.device.fs.as_ref() {
                    if fs.type_ == "crypto_LUKS" {
                        luks_to_lock.push(child.device.name.clone());
                    }
                }

                children.extend(child.device.children.iter().cloned())
            }

            for vg in vgs_to_suspend {
                let _ = crate::lvm::vg_deactivate(&vg).unwrap();
            }

            for luks in luks_to_lock {
                if let Err(why) = crate::luks::deactivate(Path::new(&luks), name) {
                    return Err(EncryptionError::Cryptsetup(why));
                }
            }
        }

        self.reload(udev, t);

        Ok(())
    }

    /// Unlock a LUKS partitition, and activate its volumes.
    pub fn luks_unlock(
        &mut self,
        device: &str,
        dm_name: &str,
        key: &[u8],
        udev: &mut UDev,
        t: &mut ACellOwner,
    ) -> Result<(), EncryptionError> {
        // Check if the LUKS partition exists.
        if self.blocks.get(device).is_none() {
            return Err(EncryptionError::DeviceNotFound);
        }

        // Decrypt the LUKS partition with libcryptsetup.
        if let Err(why) = crate::luks::activate(Path::new(device), dm_name, key) {
            return Err(EncryptionError::Cryptsetup(why));
        }

        // Ensure that any volume groups that may have been on this partition are activated.
        let _ = crate::lvm::vg_activate_all();

        // Ensure that the newly-created device map has been activated.
        self.reload(udev, t);
        while self.dm_by_dm_name(dm_name, t).is_none() {
            std::thread::sleep(std::time::Duration::from_secs(1));
            self.reload(udev, t);
        }

        Ok(())
    }
}
