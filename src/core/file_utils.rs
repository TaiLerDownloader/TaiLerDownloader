use tokio::fs::File;
use std::fs::File as SyncFile;

const FAT32_MAX_FILE_SIZE: i64 = 4_294_967_295;

pub async fn create_download_file(
    save_path: &str,
    file_size: Option<i64>,
) -> Result<File, Box<dyn std::error::Error + Send + Sync>> {
    let file = File::create(save_path).await
        .map_err(|e| format!("Failed to create file: {}", e))?;

    if let Some(size) = file_size {
        if size > FAT32_MAX_FILE_SIZE {
            return Err(format!(
                "Insufficient storage or file exceeds FAT32 4GB limit. Please ensure target path is on NTFS/exFAT partition with enough space"
            ).into());
        }

        if let Err(e) = file.set_len(size as u64).await {
            if e.kind() == std::io::ErrorKind::StorageFull {
                return Err("Insufficient disk space".into());
            }
            eprintln!("Warning: Failed to pre-allocate file space ({}), will continue downloading", e);
        }
    }

    Ok(file)
}

pub fn create_download_file_sync(
    save_path: &str,
    file_size: Option<i64>,
) -> Result<SyncFile, String> {
    let file = SyncFile::create(save_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    if let Some(size) = file_size {
        if size > FAT32_MAX_FILE_SIZE {
            return Err(format!(
                "Insufficient storage or file exceeds FAT32 4GB limit. Please ensure target path is on NTFS/exFAT partition with enough space"
            ));
        }

        if let Err(e) = file.set_len(size as u64) {
            if e.kind() == std::io::ErrorKind::StorageFull {
                return Err("Insufficient disk space".to_string());
            }
            eprintln!("Warning: Failed to pre-allocate file space ({}), will continue downloading", e);
        }
    }

    Ok(file)
}