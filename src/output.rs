//! read のファイル中心出力と stdout manifest。

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::AppError;
use crate::ir::{ReadManifest, ReadSummary};

pub struct OutputPaths {
    root: PathBuf,
    content: PathBuf,
    media_dir: PathBuf,
}

impl OutputPaths {
    /// 出力ルートと media ディレクトリを常に生成し、絶対パスとして保持する。
    pub fn resolve(input: &Path, requested: Option<&Path>) -> Result<Self, AppError> {
        let root = match requested {
            Some(path) => path.to_path_buf(),
            None => {
                let stem = input
                    .file_stem()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "out".to_string());
                PathBuf::from(format!("{stem}.officedump"))
            }
        };
        fs::create_dir_all(&root)?;
        let root = fs::canonicalize(root)?;
        let media_dir = root.join("media");
        fs::create_dir_all(&media_dir)?;
        Ok(Self {
            content: root.join("content.json"),
            root,
            media_dir,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn content(&self) -> &Path {
        &self.content
    }

    pub fn media_dir(&self) -> &Path {
        &self.media_dir
    }
}

/// JSON を一時ファイルへ書いた後に rename する。manifest はこの処理の後にだけ出す。
pub fn write_content<T: Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| AppError::Output(format!("content.json のJSON化に失敗: {error}")))?;
    let temp = path.with_file_name(format!(".content-{}.tmp", std::process::id()));
    fs::write(&temp, bytes)?;
    fs::rename(temp, path)?;
    Ok(())
}

pub fn render_read<T: Serialize>(
    paths: &OutputPaths,
    stdout: bool,
    file: String,
    format: String,
    summary: ReadSummary,
    content: &T,
) -> Result<String, AppError> {
    if stdout {
        return serde_json::to_string_pretty(content)
            .map_err(|error| AppError::Output(format!("標準出力JSON化に失敗: {error}")));
    }

    write_content(paths.content(), content)?;
    let manifest = ReadManifest {
        file,
        format,
        content: paths.content().display().to_string(),
        media_dir: paths.media_dir().display().to_string(),
        summary,
    };
    serde_json::to_string_pretty(&manifest)
        .map_err(|error| AppError::Output(format!("manifestのJSON化に失敗: {error}")))
}
