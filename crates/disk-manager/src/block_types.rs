// Copyright 2021 System76 <info@system76.com>
// SPDX-License-Identifier: LGPL-3.0-only

use crate::ACell;
use std::sync::Arc;

#[derive(Clone)]
pub enum BlockDevice {
    Partition(Arc<ACell<PartitionEntry>>),
    Disk(Arc<ACell<Disk>>),
    DeviceMap(Arc<ACell<DeviceMap>>),
}

#[derive(Clone)]
pub struct Device {
    /// A device name could be `/dev/sda1`.
    pub name: String,
    /// Number of sectors
    pub size: u64,
    pub fs: Option<FileSystem>,
    pub children: Vec<Arc<ACell<DeviceMap>>>,
}

pub struct DeviceMap {
    pub device: Device,
    pub lv_name: Option<String>,
    pub name: String,
    pub vg_name: Option<String>,
}

#[derive(Clone)]
pub struct Disk {
    pub device: Device,
    pub sector_size: u64,
    pub model: String,
    pub serial: String,
    pub table: Option<PartitionTable>,
    pub children: Vec<Arc<ACell<PartitionEntry>>>,
}

#[derive(Clone, Debug)]
pub struct FileSystem {
    pub type_: String,
    pub uuid: String,
}

pub struct PartitionEntry {
    pub device: Device,
    pub offset: u64,
    pub uuid: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartitionTable {
    Mbr,
    Gpt,
}
