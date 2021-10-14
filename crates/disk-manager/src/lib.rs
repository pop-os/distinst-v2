// Copyright 2021 System76 <info@system76.com>
// SPDX-License-Identifier: LGPL-3.0-only

#[macro_use]
extern crate serde;
#[macro_use]
extern crate thiserror;

use qcell::{TCell, TCellOwner};

mod block_types;
mod disk_manager;
pub mod luks;
pub mod lvm;
pub mod os_probe;
mod udev;

pub struct CellMarker;

pub type ACell<T> = TCell<CellMarker, T>;
pub type ACellOwner = TCellOwner<CellMarker>;

pub use self::block_types::*;
pub use self::disk_manager::*;
pub use self::udev::*;
