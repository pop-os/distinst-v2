// Copyright 2021 System76 <info@system76.com>
// SPDX-License-Identifier: LGPL-3.0-only

use cradle::prelude::*;

pub fn lv_deactivate(lv: &str) -> Result<(), cradle::Error> {
    eprintln!("deactivating LVM LV {}", lv);
    run_result!("lvchange", "-an", lv)
}

pub fn lv_activate(lv: &str) -> Result<(), cradle::Error> {
    eprintln!("activating LVM LV {}", lv);
    run_result!("lvchange", "-ay", lv)
}

pub fn vg_deactivate(vg: &str) -> Result<(), cradle::Error> {
    eprintln!("deactivating LVM VG {}", vg);
    run_result!("vgchange", "-an", vg)
}

pub fn vg_activate(vg: &str) -> Result<(), cradle::Error> {
    eprintln!("activating LVM VG {}", vg);
    run_result!("vgchange", "-ay", vg)
}

pub fn vg_activate_all() -> Result<(), cradle::Error> {
    eprintln!("activating all LVM VGs");
    run_result!("vgchange", "-ay")
}
