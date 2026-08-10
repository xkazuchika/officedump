mod error;
mod ir;
mod media;
mod range;
mod xlsx;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use error::{AppError, report};
use ir::{InspectOutput, ReadOutput, SheetDump, SheetSummary};
use range::RangeFilter;
use xlsx::XlsxPackage;

/// Office ファイルを解釈せず忠実に JSON へ分解する CLI
#[derive(Parser)]
#[command(name = "officedump", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 構造概要（シート名・寸法）だけを JSON で返す
    Inspect {
        /// 対象の xlsx ファイル
        file: PathBuf,
    },
    /// 分解した JSON IR を標準出力に出す
    Read {
        /// 対象の xlsx ファイル
        file: PathBuf,
        /// 対象シート名（省略時は全シート）
        #[arg(long)]
        sheet: Option<String>,
        /// セル範囲（例: A1:F50, A:C, 1:30）
        #[arg(long)]
        range: Option<String>,
        /// メディア等の出力先（省略時は <ファイル名>.officedump/）
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Inspect { file } => run_inspect(&file),
        Command::Read {
            file,
            sheet,
            range,
            out,
        } => run_read(&file, sheet.as_deref(), range.as_deref(), out.as_deref()),
    };
    if let Err(e) = result {
        report(&e);
    }
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// "xl/worksheets/sheet1.xml" -> "xl/worksheets"
fn dirname(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

/// "xl/worksheets/sheet1.xml" -> "sheet1.xml"
fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// "xl/worksheets/sheet1.xml" -> "xl/worksheets/_rels/sheet1.xml.rels"
fn rels_path_of(part_path: &str) -> String {
    format!("{}/_rels/{}.rels", dirname(part_path), basename(part_path))
}

fn open_package(file: &Path) -> Result<(XlsxPackage, Vec<xlsx::SheetMeta>), AppError> {
    let mut pkg = XlsxPackage::open(file)?;
    let workbook_xml = pkg.read_part("xl/workbook.xml")?;
    let rels_xml = pkg.read_part("xl/_rels/workbook.xml.rels")?;
    let rels = xlsx::parse_rels(&rels_xml)?;
    let sheets = xlsx::parse_workbook(&workbook_xml, &rels)?;
    Ok((pkg, sheets))
}

fn run_inspect(file: &Path) -> Result<(), AppError> {
    let (mut pkg, sheets) = open_package(file)?;
    let mut summaries = Vec::new();
    for sheet in &sheets {
        let xml = pkg.read_part(&sheet.path)?;
        // design: dimension 属性を優先し、欠落・不正時のみ走査で確定する
        let (rows, cols) =
            match xlsx::sheet_dimension(&xml).and_then(|d| xlsx::extent_from_dimension(&d)) {
                Some(rc) => rc,
                None => xlsx::scan_extent(&xml)?,
            };
        summaries.push(SheetSummary {
            name: sheet.name.clone(),
            rows,
            cols,
        });
    }
    let out = InspectOutput {
        file: file_label(file),
        format: "xlsx".to_string(),
        sheets: summaries,
    };
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    Ok(())
}

fn default_out_dir(file: &Path) -> PathBuf {
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "out".to_string());
    PathBuf::from(format!("{stem}.officedump"))
}

fn run_read(
    file: &Path,
    sheet_filter: Option<&str>,
    range: Option<&str>,
    out: Option<&Path>,
) -> Result<(), AppError> {
    let filter = match range {
        Some(r) => range::parse_range(r)?,
        None => RangeFilter::all(),
    };

    let (mut pkg, mut sheets) = open_package(file)?;
    if let Some(name) = sheet_filter {
        sheets.retain(|s| s.name == name);
        if sheets.is_empty() {
            return Err(AppError::SheetNotFound(name.to_string()));
        }
    }

    let shared = match pkg.read_part_opt("xl/sharedStrings.xml")? {
        Some(xml) => xlsx::parse_shared_strings(&xml)?,
        None => Vec::new(),
    };
    let styles = match pkg.read_part_opt("xl/styles.xml")? {
        Some(xml) => xlsx::parse_styles(&xml)?,
        None => xlsx::Styles::default(),
    };

    let out_dir = out
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| default_out_dir(file));
    let extracted = pkg.extract_media(&out_dir)?;

    let mut dumps = Vec::new();
    let mut anchors_with_rels = Vec::new();

    for sheet in &sheets {
        let xml = pkg.read_part(&sheet.path)?;
        let parsed = xlsx::parse_worksheet(&xml, &filter, &shared, &styles)?;

        if let Some(drid) = &parsed.drawing_rid
            && let Some(rels_xml) = pkg.read_part_opt(&rels_path_of(&sheet.path))?
        {
            let sheet_rels = xlsx::parse_rels(&rels_xml)?;
            if let Some(drawing_target) = sheet_rels.get(drid) {
                let drawing_path = xlsx::resolve_target(dirname(&sheet.path), drawing_target);
                let drawing_xml = pkg.read_part(&drawing_path)?;
                let drawing_rels_xml = pkg.read_part(&rels_path_of(&drawing_path))?;
                let drawing_rels = xlsx::parse_rels(&drawing_rels_xml)?;
                for draft in media::parse_drawing(&drawing_xml)? {
                    anchors_with_rels.push((sheet.name.clone(), draft, drawing_rels.clone()));
                }
            }
        }

        dumps.push(SheetDump {
            name: sheet.name.clone(),
            dimension: parsed.dimension,
            merged_cells: parsed.merged,
            cells: parsed.cells,
            unhandled: parsed.unhandled,
        });
    }

    let media_items = media::build_media_items(&extracted, anchors_with_rels)?;

    let output = ReadOutput {
        file: file_label(file),
        format: "xlsx".to_string(),
        sheets: dumps,
        media: media_items,
    };
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
    Ok(())
}
