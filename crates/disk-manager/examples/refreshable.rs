// Copyright 2021 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

use pop_disk_manager::os_probe::boot_entries;
use pop_disk_manager::*;

fn main() {
    let context = libudev::Context::new().unwrap();
    let mut enumerator = libudev::Enumerator::new(&context).unwrap();

    enumerator.match_subsystem("block").unwrap();

    let mut udev = UDev {
        context,
        enumerator,
    };

    let mut t = ACellOwner::new();

    let dm = devicemapper::DM::new().unwrap();

    let mut disk_manager = DiskManager::new(dm);

    disk_manager.reload(&mut udev, &mut t);

    for entry in boot_entries(&disk_manager, &t) {
        println!("{:?}", entry);
        for (devname, block) in &disk_manager.blocks {
            match block {
                BlockDevice::Disk(disk) => {
                    let disk = disk.ro(&t);
                    if let Some(fs) = disk.device.fs.as_ref() {
                        if fs.uuid == entry.uuid {
                            eprintln!("  Entry is on {}", devname);
                        }
                    }
                }
                BlockDevice::Partition(part) => {
                    let part = part.ro(&t);
                    if let Some(fs) = part.device.fs.as_ref() {
                        if fs.uuid == entry.uuid {
                            eprintln!("  Entry is on {}", devname);
                        }
                    }
                }
                BlockDevice::DeviceMap(dm) => {
                    let dm = dm.ro(&t);
                    if let Some(fs) = dm.device.fs.as_ref() {
                        if fs.uuid == entry.uuid {
                            eprintln!("  Entry is on {}", devname);
                        }
                    }
                }
            }
        }
    }
}
