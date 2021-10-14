// Copyright 2021 System76 <info@system76.com>
// SPDX-License-Identifier: LGPL-3.0-only

use crate::block_types::BlockDevice;
use crate::disk_manager::DiskManager;
use crate::ACellOwner;
use os_release::OsRelease;
use std::fs;
use std::path::{Path, PathBuf};
use sys_mount::scoped_mount;
use zvariant::derive::Type;

#[derive(Clone, Debug)]
pub struct LinuxOS {
    pub partition: PathBuf,
    pub release: OsRelease,
}

#[derive(Clone, Debug, Deserialize, Serialize, Type)]
pub struct OsEntry {
    pub path: String,
    pub uuid: String,
}

/// Search vfat partitions for systemd-boot entries.
pub fn boot_entries(disk_manager: &DiskManager, t: &ACellOwner) -> Vec<OsEntry> {
    let target_mount = Path::new("/tmp/distinst_disk_probe");

    let entry_path = target_mount.join("loader/entries/");

    if target_mount.exists() {
        let _ = sys_mount::unmount(&target_mount, sys_mount::UnmountFlags::DETACH);
    } else {
        let _ = fs::create_dir(target_mount);
    }

    for (devname, block) in disk_manager.blocks.iter() {
        let device = match block {
            BlockDevice::Partition(entry) => &entry.ro(t).device,
            BlockDevice::Disk(disk) => &disk.ro(t).device,
            _ => continue,
        };

        if let Some(fs) = device.fs.as_ref() {
            if fs.type_ != "vfat" {
                continue;
            }

            return locate_boot_entries(&Path::new(devname), target_mount, &entry_path);
        }
    }

    Vec::new()
}

/// Locate a Linux installation on a partition by its /etc/os-release.
pub fn linux(partition: &Path) -> Option<LinuxOS> {
    let target_mount = Path::new("/tmp/distinst_os_probe");
    let _ = sys_mount::unmount(&target_mount, sys_mount::UnmountFlags::DETACH);
    scoped_mount(partition, target_mount, move || {
        let release = ward::ward!(OsRelease::new_from(&*target_mount.join("etc/os-release")).ok(), else {
            return None;
        });

        Some(LinuxOS { partition: partition.to_owned(), release })
    }).ok().flatten()
}

fn locate_boot_entries(partition: &Path, mount_at: &Path, entry_path: &Path) -> Vec<OsEntry> {
    let result = scoped_mount(partition, mount_at, move || {
        let dir = match fs::read_dir(entry_path) {
            Ok(dir) => dir,
            Err(_) => return None,
        };

        let entries = dir
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                // let file_name = ward::ward!(path.file_name().and_then(OsStr::to_str), else {
                //     return None;
                // });

                let entry = ward::ward!(fs::read_to_string(&path).ok(), else { return None });

                entry
                    .lines()
                    .find_map(|l| l.strip_prefix("options "))
                    .and_then(|line| {
                        line.split_ascii_whitespace()
                            .find_map(|f| f.strip_prefix("root=UUID="))
                            .map(|uuid| String::from(uuid))
                    })
                    .map(|uuid| OsEntry {
                        path: partition.to_string_lossy().to_owned().to_string(),
                        uuid,
                    })
            })
            .collect::<Vec<OsEntry>>();

        Some(entries)
    });

    result.ok().flatten().unwrap_or_default()
}
