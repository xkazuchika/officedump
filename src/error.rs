use std::fmt;

/// アプリ全体のエラー。機械可読な JSON 報告のための種別（kind）を持つ。
#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    InvalidXlsx(String),
    MissingPart(String),
    Xml(String),
    InvalidRange(String),
    SheetNotFound(String),
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Io(_) => "io_error",
            AppError::InvalidXlsx(_) => "invalid_xlsx",
            AppError::MissingPart(_) => "missing_part",
            AppError::Xml(_) => "xml_error",
            AppError::InvalidRange(_) => "invalid_range",
            AppError::SheetNotFound(_) => "sheet_not_found",
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "I/Oエラー: {e}"),
            AppError::InvalidXlsx(m) => write!(f, "xlsx として不正: {m}"),
            AppError::MissingPart(p) => write!(f, "必須パートが見つかりません: {p}"),
            AppError::Xml(m) => write!(f, "XML パースエラー: {m}"),
            AppError::InvalidRange(m) => write!(f, "範囲指定が不正: {m}"),
            AppError::SheetNotFound(m) => write!(f, "シートが見つかりません: {m}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

/// エラーを JSON で標準エラーへ出力し、非ゼロコードで終了する。
pub fn report(err: &AppError) -> ! {
    let body = serde_json::json!({
        "error": { "kind": err.kind(), "message": err.to_string() }
    });
    eprintln!("{body}");
    std::process::exit(1);
}
