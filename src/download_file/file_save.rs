use super::error::Result;
use crate::download_file::error::DownloadError::SaveFileFinish;
use aqueue::Actor;
use std::io::SeekFrom;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

/// file save actor
pub(crate) struct FileSave {
    save_path: PathBuf,
    real_path: PathBuf,
    len: u64,
    file: Option<File>,
}

impl FileSave {
    /// create file save actor
    #[inline]
    pub fn create(real_path: PathBuf, len: u64) -> Result<Actor<FileSave>> {
        let save_path = real_path.with_extension("dd");
        if save_path.exists() {
            std::fs::remove_file(save_path.as_path())?;
            log::trace!("delete old file:{:?}", save_path);
        }
        Ok(Actor::new(Self {
            save_path,
            real_path,
            len,
            file: None,
        }))
    }

    #[inline]
    async fn init(&mut self) -> Result<()> {
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(self.save_path.as_path())
            .await?;
        file.set_len(self.len).await?;
        log::trace!("create file:{:?} size:{}", self.save_path, self.len);
        self.file = Some(file);
        Ok(())
    }

    /// write data to file,need offset
    #[inline]
    async fn write_all_by_offset(&mut self, data: &[u8], offset: u64) -> Result<()> {
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| SaveFileFinish(self.real_path.to_string_lossy().to_string()))?;
        file.seek(SeekFrom::Start(offset)).await?;
        file.write_all(data).await?;
        Ok(())
    }

    /// write data
    #[inline]
    async fn write_all(&mut self, data: &[u8]) -> Result<()> {
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| SaveFileFinish(self.real_path.to_string_lossy().to_string()))?;
        file.write_all(data).await?;
        Ok(())
    }

    /// finish save file and rename real name
    #[inline]
    async fn finish(&mut self) -> Result<()> {
        if let Some(mut file) = self.file.take() {
            file.flush().await?;
            drop(file);
            std::fs::rename(self.save_path.as_path(), self.real_path.as_path())?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
pub(crate) trait IFileSave {
    /// init file
    async fn init(&self) -> Result<()>;
    /// write data
    async fn write_all(&self, data: &[u8]) -> Result<()>;
    /// write data to file,need offset
    async fn write_all_by_offset(&self, data: &[u8], offset: u64) -> Result<()>;
    /// finish save file and rename real name
    async fn finish(&self) -> Result<()>;
    /// get save file path
    fn get_save_file_path(&self) -> String;
    /// get real file save path
    fn get_real_file_path(&self) -> String;
}

#[async_trait::async_trait]
impl IFileSave for Actor<FileSave> {
    #[inline]
    async fn init(&self) -> Result<()> {
        self.inner_call(|inner| async move { inner.get_mut().init().await })
            .await
    }

    #[inline]
    async fn write_all(&self, data: &[u8]) -> Result<()> {
        self.inner_call(|inner| async move { inner.get_mut().write_all(data).await })
            .await
    }

    #[inline]
    async fn write_all_by_offset(&self, data: &[u8], offset: u64) -> Result<()> {
        self.inner_call(
            |inner| async move { inner.get_mut().write_all_by_offset(data, offset).await },
        )
        .await
    }
    #[inline]
    async fn finish(&self) -> Result<()> {
        self.inner_call(|inner| async move { inner.get_mut().finish().await })
            .await
    }
    #[inline]
    fn get_save_file_path(&self) -> String {
        unsafe { self.deref_inner().save_path.to_string_lossy().to_string() }
    }
    #[inline]
    fn get_real_file_path(&self) -> String {
        unsafe { self.deref_inner().real_path.to_string_lossy().to_string() }
    }
}
