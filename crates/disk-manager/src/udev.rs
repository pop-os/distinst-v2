// Copyright 2021 System76 <info@system76.com>
// SPDX-License-Identifier: LGPL-3.0-only

use crate::block_types::*;
use crate::disk_manager::DiskManager;
use crate::{ACell, ACellOwner};
use libudev::Device as UDevice;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct UDev {
    pub context: libudev::Context,
    pub enumerator: libudev::Enumerator,
}

impl UDev {
    /// Appends relevant information about this device to the disk manager.
    pub fn append(&self, dm: &mut DiskManager, device: &UDevice, t: &mut ACellOwner) {
        match device.devtype().and_then(OsStr::to_str) {
            Some("disk") => match property(device, "DM_NAME") {
                Some(dm_name) => self.append_dm(dm, device, dm_name.to_owned(), t),
                None => self.append_disk(dm, device, t),
            },
            Some("partition") => self.append_partition(dm, device, t),
            _ => (),
        }
    }

    /// Append a device which we have determined to be a physical disk.
    fn append_disk(&self, dm: &mut DiskManager, device: &UDevice, _t: &mut ACellOwner) {
        let dev = ward::ward!(disk_manager_device(device), else { return });

        if dev.name.contains("loop") {
            return;
        }

        let table = property(device, "ID_PART_TABLE_TYPE").and_then(|table| match table {
            "gpt" => Some(PartitionTable::Gpt),
            "mbr" => Some(PartitionTable::Mbr),
            _ => None,
        });

        let sector_size = match device
            .attribute_value("queue/logical_block_size")
            .and_then(OsStr::to_str)
        {
            Some(size) => match size.parse::<u64>() {
                Ok(size) => size,
                Err(_) => {
                    eprintln!(
                        "{}: does not contain a valid sector size: {}",
                        dev.name, size
                    );
                    return;
                }
            },
            None => {
                eprintln!("{}: does not contain a sector size", dev.name);
                return;
            }
        };

        dm.blocks.insert(
            dev.name.clone(),
            BlockDevice::Disk(Arc::new(ACell::new(Disk {
                device: dev,
                table,
                sector_size,
                model: property(device, "ID_MODEL").unwrap_or_default().to_owned(),
                serial: property(device, "ID_SERIAL").unwrap_or_default().to_owned(),
                children: Vec::new(),
            }))),
        );
    }

    /// Append a device which we have determined to be a physical partition.
    fn append_partition(&self, dm: &mut DiskManager, dev: &UDevice, t: &mut ACellOwner) {
        let device = ward::ward!(disk_manager_device(dev), else {
            eprintln!("partition without device information");
            return;
        });

        let offset = ward::ward!(property(dev, "ID_PART_ENTRY_OFFSET"), else {
            eprintln!("{}: lacks ID_PART_ENTRY_OFFSET", device.name);
            return;
        });

        let uuid = ward::ward!(property(dev, "ID_PART_ENTRY_UUID"), else {
            eprintln!("{}: lacks ID_PART_ENTRY_UUID", device.name);
            return;
        });

        let syspath = ward::ward!(dev.syspath(), else {
            eprintln!("{}: partition device without syspath", device.name);
            return;
        });

        let parent = ward::ward!(parents(syspath).next(), else {
            eprintln!("{}: partition lacks parent", device.name);
            return;
        });

        let parent_dev = match UDevice::from_syspath(&self.context, &parent) {
            Ok(dev) => dev,
            Err(why) => {
                eprintln!("{:?}: libudev device without syspath: {}", parent, why);
                return;
            }
        };

        let parent_devname = ward::ward!(property(&parent_dev, "DEVNAME"), else {
            eprintln!("{:?}: libudev device without DEVNAME", parent);
            return;
        });

        let devname = device.name.clone();

        let partition = Arc::new(ACell::new(PartitionEntry {
            offset: offset.parse::<u64>().unwrap_or_default(),
            uuid: uuid.to_owned(),
            device,
        }));

        dm.blocks
            .insert(devname, BlockDevice::Partition(partition.clone()));

        let parent_block = ward::ward!(dm.blocks.get_mut(parent_devname), else {
            eprintln!("{:?}: not found in disk manager", parent);
            return;
        });

        match parent_block {
            BlockDevice::Disk(disk) => {
                disk.rw(t).children.push(partition);
            }
            _ => {
                eprintln!("parent is not a disk");
            }
        }
    }

    /// Append a device which we have determined to be a device map.
    fn append_dm(&self, dm: &mut DiskManager, dev: &UDevice, dm_name: String, t: &mut ACellOwner) {
        let device = ward::ward!(disk_manager_device(dev), else {
            eprintln!("partition without device information");
            return;
        });

        let syspath = ward::ward!(dev.syspath(), else {
            eprintln!("{}: partition device without syspath", device.name);
            return;
        });

        let parent = ward::ward!(parents(syspath).next(), else {
            eprintln!("{}: partition lacks parent", device.name);
            return;
        });

        let parent_dev = match UDevice::from_syspath(&self.context, &parent) {
            Ok(dev) => dev,
            Err(why) => {
                eprintln!("{:?}: libudev device without syspath: {}", parent, why);
                return;
            }
        };

        let parent_devname = ward::ward!(property(&parent_dev, "DEVNAME"), else {
            eprintln!("{:?}: libudev device without DEVNAME", parent);
            return;
        });

        let lv_name = property(dev, "DM_LV_NAME");

        let vg_name = property(dev, "DM_VG_NAME");

        let devname = device.name.clone();

        let device_map = Arc::new(ACell::new(DeviceMap {
            device,
            lv_name: lv_name.map(String::from),
            name: dm_name,
            vg_name: vg_name.map(String::from),
        }));

        dm.blocks
            .insert(devname.clone(), BlockDevice::DeviceMap(device_map.clone()));

        match dm.blocks.get_mut(parent_devname) {
            Some(BlockDevice::DeviceMap(dm)) => {
                dm.rw(t).device.children.push(device_map);
            }
            Some(BlockDevice::Partition(part)) => {
                part.rw(t).device.children.push(device_map);
            }
            Some(BlockDevice::Disk(disk)) => disk.rw(t).device.children.push(device_map),
            None => {
                eprintln!("{}: could not find parent block", devname)
            }
        }
    }
}

/// Automatically convert `UDevice` properties to Rust strings.
fn property<'a>(device: &'a UDevice, property: &str) -> Option<&'a str> {
    device.property_value(property).and_then(OsStr::to_str)
}

/// Get device-specific information from a `UDevice`.
fn disk_manager_device(device: &UDevice) -> Option<Device> {
    let name = device.property_value("DEVNAME")?;
    let size = device.attribute_value("size")?;

    let name = name.to_str()?.to_owned();
    let size = size.to_str()?.parse::<u64>().ok()?;
    let fs = disk_manager_filesystem(device);

    Some(Device {
        name,
        size,
        fs,
        children: Vec::new(),
    })
}

/// Get filesystem-specific information from a `UDevice`.
fn disk_manager_filesystem(device: &UDevice) -> Option<FileSystem> {
    if let Some(type_) = device.property_value("ID_FS_TYPE") {
        let uuid = device.property_value("ID_FS_UUID").and_then(OsStr::to_str);

        if let Some((type_, uuid)) = type_.to_str().zip(uuid) {
            let type_ = type_.to_owned();
            let uuid = uuid.to_owned();
            Some(FileSystem { type_, uuid })
        } else {
            None
        }
    } else {
        None
    }
}

/// Locate the parents of the given device.
///
/// Some devices define their parents under `{DEV}/slaves`, while others can be determined
/// by checking if the parent directory contains a `queue` sub-directory.
pub fn parents(device_path: &Path) -> impl Iterator<Item = PathBuf> {
    let parents: Box<dyn Iterator<Item = PathBuf>> = match device_path.join("slaves").read_dir() {
        Ok(parents) => {
            let iterator = parents
                .filter_map(Result::ok)
                .filter_map(|parent| parent.path().canonicalize().ok())
                .filter(|path| path.components().any(|c| c.as_os_str() == "block"));

            Box::new(iterator)
        }
        Err(_) => Box::new(std::iter::empty()),
    };

    let parent = device_path.parent().and_then(|parent| {
        if parent.join("queue").exists() {
            Some(parent.to_owned())
        } else {
            None
        }
    });

    parents.chain(parent)
}
