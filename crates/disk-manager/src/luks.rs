// Copyright 2021 System76 <info@system76.com>
// SPDX-License-Identifier: LGPL-3.0-only

use cryptsetup::{
    CryptActivateFlags, CryptDeactivateFlags, CryptInit, CryptVolumeKeyFlags, EncryptionFormat,
};
use libcryptsetup_rs as cryptsetup;
use std::path::Path;

pub fn encrypt(path: &Path, passphrase: &[u8]) -> cryptsetup::Result<()> {
    eprintln!("creating LUKS device on {:?}", path);
    let mut device = CryptInit::init(path)?;

    device.context_handle().format::<()>(
        EncryptionFormat::Luks2,
        ("aes", "xts-plain"),
        None,
        libcryptsetup_rs::Either::Right(256 / 8),
        None,
    )?;

    device
        .keyslot_handle()
        .add_by_key(None, None, passphrase, CryptVolumeKeyFlags::empty())?;

    Ok(())
}

pub fn activate(path: &Path, name: &str, passphrase: &[u8]) -> cryptsetup::Result<()> {
    eprintln!(
        "activating LUKS device {:?}, with DM_NAME of {}",
        path, name
    );
    let mut device = CryptInit::init(path)?;
    device
        .context_handle()
        .load::<()>(Some(EncryptionFormat::Luks2), None)?;

    device.activate_handle().activate_by_passphrase(
        Some(name),
        None,
        passphrase,
        CryptActivateFlags::empty(),
    )?;
    Ok(())
}

pub fn deactivate(path: &Path, name: &str) -> cryptsetup::Result<()> {
    eprintln!(
        "deactivating LUKS device {:?}, which has DM_NAME of {}",
        path, name
    );
    let mut device = CryptInit::init(path)?;
    device
        .context_handle()
        .load::<()>(Some(EncryptionFormat::Luks2), None)?;

    device
        .activate_handle()
        .deactivate(name, CryptDeactivateFlags::empty())?;
    Ok(())
}
