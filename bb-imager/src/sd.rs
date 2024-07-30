//! Provide functionality to flash images to sd card

use std::io::{Read, Seek, Write};

use crate::DownloadFlashingStatus;
use crate::{error::Result, BUF_SIZE};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Sha256 verification error")]
    Sha256VerificationError,
    #[error("Failed to get removable flash drives")]
    DriveFetchError,
}

pub(crate) fn flash(
    mut img: crate::img::OsImage,
    mut sd: std::fs::File,
    chan: &std::sync::mpsc::Sender<DownloadFlashingStatus>,
) -> Result<()> {
    let size = img.size();

    let mut buf = [0u8; BUF_SIZE];
    let mut pos = 0;

    let _ = chan.send(DownloadFlashingStatus::FlashingProgress(0.0));

    loop {
        let count = img.read(&mut buf)?;
        pos += count;

        let _ = chan.send(DownloadFlashingStatus::FlashingProgress(
            pos as f32 / size as f32,
        ));

        if count == 0 {
            break;
        }

        sd.write_all(&buf[..count])?;
    }

    if let Some(sha256) = img.sha256() {
        let _ = chan.send(DownloadFlashingStatus::VerifyingProgress(0.0));

        sd.seek(std::io::SeekFrom::Start(0))?;
        let hash = crate::util::sha256_file_fixed_progress(sd, size, chan)?;

        if hash != sha256 {
            return Err(Error::Sha256VerificationError.into());
        }
    }

    Ok(())
}

// pub fn format(dev: &Path) -> io::Result<()> {
//     let disk = std::fs::OpenOptions::new()
//         .read(true)
//         .write(true)
//         .open(dev)?;
//     fatfs::format_volume(disk, fatfs::FormatVolumeOptions::new())
// }

#[cfg(target_os = "linux")]
pub async fn destinations(
    state: &crate::State,
) -> Result<std::collections::HashSet<crate::Destination>> {
    use std::collections::HashSet;

    let block_devs = state
        .dbus_client
        .manager()
        .get_block_devices(Default::default())
        .await?
        .into_iter()
        .map(|x| state.dbus_client.object(x).unwrap())
        .collect::<Vec<_>>();

    let mut ans = HashSet::new();

    for obj in block_devs {
        if let Ok(block) = obj.block().await {
            if let Ok(drive) = state.dbus_client.drive_for_block(&block).await {
                if drive.removable().await.unwrap() && drive.media_removable().await.unwrap() {
                    let block = state
                        .dbus_client
                        .block_for_drive(&drive, true)
                        .await
                        .unwrap()
                        .into_inner()
                        .path()
                        .to_owned();
                    ans.insert(crate::Destination::sd_card(
                        drive.id().await?,
                        drive.size().await?,
                        block.into(),
                    ));
                }
            }
        }
    }

    Ok(ans)
}

#[cfg(windows)]
pub async fn destinations(
    state: &crate::State,
) -> Result<std::collections::HashSet<crate::Destination>> {
    use std::collections::HashSet;

    Ok(HashSet::new())
}
