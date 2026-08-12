//! Office パッケージ（zip コンテナ）への共通アクセス層。

use std::fs::File;
use std::io::Read;
use std::path::Path;

use zip::ZipArchive;

use crate::error::AppError;

/// 対応する Office 形式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficeFormat {
    Xlsx,
    Docx,
}

impl OfficeFormat {
    pub fn media_prefix(&self) -> &'static str {
        match self {
            OfficeFormat::Xlsx => "xl/media/",
            OfficeFormat::Docx => "word/media/",
        }
    }

    /// 形式に対応した「パッケージ不正」エラーを生成する。
    pub fn invalid(&self, msg: String) -> AppError {
        match self {
            OfficeFormat::Xlsx => AppError::InvalidXlsx(msg),
            OfficeFormat::Docx => AppError::InvalidDocx(msg),
        }
    }
}

/// 形式共通の zip アクセス層。
pub struct OfficePackage {
    zip: ZipArchive<File>,
    format: OfficeFormat,
}

impl OfficePackage {
    pub fn open(path: &Path, format: OfficeFormat) -> Result<Self, AppError> {
        let file = File::open(path)?;
        let zip = ZipArchive::new(file)
            .map_err(|e| format.invalid(format!("zip として開けません: {e}")))?;
        Ok(Self { zip, format })
    }

    /// 指定パートを文字列として読む。存在しなければ MissingPart。
    pub fn read_part(&mut self, name: &str) -> Result<String, AppError> {
        let fmt = self.format;
        match self.zip.by_name(name) {
            Ok(mut f) => {
                let mut buf = String::new();
                f.read_to_string(&mut buf).map_err(|_| {
                    fmt.invalid(format!("{name} が UTF-8 テキストとして読めません"))
                })?;
                Ok(buf)
            }
            Err(_) => Err(AppError::MissingPart(name.to_string())),
        }
    }

    /// 存在しなければ Ok(None) を返す版。
    pub fn read_part_opt(&mut self, name: &str) -> Result<Option<String>, AppError> {
        match self.read_part(name) {
            Ok(s) => Ok(Some(s)),
            Err(AppError::MissingPart(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// メディアを `<out_dir>/media/` に元バイナリのまま抽出し、
    /// JSON からの参照パス（"media/<name>"）の一覧を返す。
    pub fn extract_media(&mut self, out_dir: &Path) -> Result<Vec<String>, AppError> {
        let fmt = self.format;
        let prefix = fmt.media_prefix();
        let names: Vec<String> = self
            .zip
            .file_names()
            .filter(|n| n.starts_with(prefix) && !n.ends_with('/'))
            .map(|s| s.to_string())
            .collect();
        let mut refs = Vec::new();
        if names.is_empty() {
            return Ok(refs);
        }
        let media_dir = out_dir.join("media");
        std::fs::create_dir_all(&media_dir)?;
        for name in names {
            let base = name.rsplit('/').next().unwrap_or(&name).to_string();
            let mut buf = Vec::new();
            self.zip
                .by_name(&name)
                .map_err(|e| fmt.invalid(format!("{name} を開けません: {e}")))?
                .read_to_end(&mut buf)?;
            std::fs::write(media_dir.join(&base), &buf)?;
            refs.push(format!("media/{base}"));
        }
        refs.sort();
        Ok(refs)
    }
}
