// Copyright 2021 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

#[macro_use]
extern crate fomat_macros;

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
    display(&disk_manager, &t);
    display(&disk_manager, &t);

    disk_manager
        .luks_lock("/dev/sda1", "crypttest", &mut udev, &mut t)
        .unwrap();

    display(&disk_manager, &t);
    disk_manager
        .luks_unlock("/dev/sda1", "crypttest", b"testing", &mut udev, &mut t)
        .unwrap();

    display(&disk_manager, &t);
    for device in udev.enumerator.scan_devices().unwrap() {
        if let Some(path) = device.syspath() {
            println!("Device: {:?}", path);
            println!("  Properties:");
            for property in device.properties() {
                println!("    {:?} = {:?}", property.name(), property.value());
            }
            println!("  Attributes:");
            for attribute in device.attributes() {
                println!("    {:?} = {:?}", attribute.name(), attribute.value());
            }
        }
    }
}

fn display(disk_manager: &DiskManager, t: &ACellOwner) {
    fn display_dm(dm: &DeviceMap, level: u32, t: &ACellOwner) {
        let indent = "  ".repeat(level as usize);

        pintln!(
            "        " (indent) "Child: " (dm.device.name) "\n"
            "          " (indent) "Size: " (dm.device.size) "\n"
            "          " (indent) "DM Name: " (dm.name) "\n"
            "          " (indent) "LV Name: " [dm.lv_name] "\n"
            "          " (indent) "VG NAME: " [dm.vg_name]
            if let Some(fs) = dm.device.fs.as_ref() {
                "\n          " (indent) "FS: " [fs]
            }
        );

        for child in dm.device.children.iter() {
            display_dm(child.ro(t), level + 1, t);
        }
    }

    for (devname, device) in &disk_manager.blocks {
        let disk = match device {
            BlockDevice::Disk(disk) => disk,
            _ => continue,
        };

        let disk = disk.ro(t);

        pintln!(
            "Disk: " (devname) "\n"
            "  Size: " (disk.device.size) "\n"
            "  Sector Size: " (disk.sector_size) "\n"
            "  Model: " (disk.model) "\n"
            "  Serial: " (disk.serial) "\n"
            "  Table: " [disk.table] "\n"
            "  FS: " [disk.device.fs]
        );

        for child in disk.device.children.iter() {
            display_dm(child.ro(t), 0, t);
        }

        for child in &disk.children {
            let part = child.ro(t);

            pintln!(
                "    Partition: " (part.device.name) "\n"
                "      Size: " (part.device.size) "\n"
                "      Offset: " (part.offset) "\n"
                "      PartUUID: " (part.uuid) "\n"
                "      FS: " [part.device.fs]
            );

            for child in part.device.children.iter() {
                display_dm(child.ro(t), 0, t);
            }
        }
    }
}
