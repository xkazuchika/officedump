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
    Pptx,
}

impl OfficeFormat {
    pub fn media_prefix(&self) -> &'static str {
        match self {
            OfficeFormat::Xlsx => "xl/media/",
            OfficeFormat::Docx => "word/media/",
            OfficeFormat::Pptx => "ppt/media/",
        }
    }

    /// 形式に対応した「パッケージ不正」エラーを生成する。
    pub fn invalid(&self, msg: String) -> AppError {
        match self {
            OfficeFormat::Xlsx => AppError::InvalidXlsx(msg),
            OfficeFormat::Docx => AppError::InvalidDocx(msg),
            OfficeFormat::Pptx => AppError::InvalidPptx(msg),
        }
    }

    /// zip 内のメディアパスから、出力先 `media/` 以下の参照パスを生成する。
    /// `..` による逸脱や空・カレント成分は正規化し、範囲外なら `None` を返す。
    pub fn media_ref(&self, part_path: &str) -> Option<String> {
        let rest = part_path.strip_prefix(self.media_prefix())?;
        let mut comps: Vec<&str> = Vec::new();
        for c in rest.split('/') {
            match c {
                "" | "." => {}
                ".." => {
                    comps.pop()?;
                }
                c => comps.push(c),
            }
        }
        if comps.is_empty() {
            return None;
        }
        Some(format!("media/{}", comps.join("/")))
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
            let json_ref = self.format.media_ref(&name).ok_or_else(|| {
                fmt.invalid(format!("メディアパスが不正です: {name}"))
            })?;
            let sub = json_ref
                .strip_prefix("media/")
                .expect("media_ref は media/ プレフィックスを持つ");
            let out_path = media_dir.join(sub);
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut buf = Vec::new();
            self.zip
                .by_name(&name)
                .map_err(|e| fmt.invalid(format!("{name} を開けません: {e}")))?
                .read_to_end(&mut buf)?;
            std::fs::write(&out_path, &buf)?;
            refs.push(json_ref);
        }
        refs.sort();
        Ok(refs)
    }
}
