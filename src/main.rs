mod docx;
mod error;
mod ir;
mod media;
mod package;
mod range;
mod xlsx;
mod xmlutil;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use error::{AppError, report};
use ir::{
    DocxInspectOutput, DocxReadOutput, DocxSectionSummary, InspectOutput, MediaAnchor, ReadOutput,
    SheetDump, SheetSummary,
};
use package::{OfficeFormat, OfficePackage};
use range::RangeFilter;

/// Office ファイルを解釈せず忠実に JSON へ分解する CLI
#[derive(Parser)]
#[command(name = "officedump", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 構造概要を JSON で返す
    Inspect { file: PathBuf },
    /// 分解した JSON IR を標準出力に出す
    Read {
        file: PathBuf,
        /// xlsx の対象シート名（省略時は全シート）
        #[arg(long)]
        sheet: Option<String>,
        /// xlsx のセル範囲（例: A1:F50, A:C, 1:30）
        #[arg(long)]
        range: Option<String>,
        /// docx の本文ブロック範囲（例: 1:10）
        #[arg(long)]
        para: Option<String>,
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
            para,
            out,
        } => run_read(
            &file,
            sheet.as_deref(),
            range.as_deref(),
            para.as_deref(),
            out.as_deref(),
        ),
    };
    if let Err(e) = result {
        report(&e);
    }
}

fn detect_format(file: &Path) -> Result<OfficeFormat, AppError> {
    match file.extension().and_then(|e| e.to_str()) {
        Some("xlsx") => Ok(OfficeFormat::Xlsx),
        Some("docx") => Ok(OfficeFormat::Docx),
        Some(ext) => Err(AppError::UnsupportedFormat(format!(".{ext}"))),
        None => Err(AppError::UnsupportedFormat("拡張子なし".to_string())),
    }
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn dirname(path: &str) -> &str {
    path.rfind('/').map(|i| &path[..i]).unwrap_or("")
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn rels_path_of(part_path: &str) -> String {
    format!("{}/_rels/{}.rels", dirname(part_path), basename(part_path))
}

fn default_out_dir(file: &Path) -> PathBuf {
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "out".to_string());
    PathBuf::from(format!("{stem}.officedump"))
}

fn run_inspect(file: &Path) -> Result<(), AppError> {
    match detect_format(file)? {
        OfficeFormat::Xlsx => run_inspect_xlsx(file),
        OfficeFormat::Docx => run_inspect_docx(file),
    }
}

fn run_read(
    file: &Path,
    sheet: Option<&str>,
    range: Option<&str>,
    para: Option<&str>,
    out: Option<&Path>,
) -> Result<(), AppError> {
    match detect_format(file)? {
        OfficeFormat::Xlsx => {
            if para.is_some() {
                return Err(AppError::Usage("--para は docx 専用です".to_string()));
            }
            run_read_xlsx(file, sheet, range, out)
        }
        OfficeFormat::Docx => {
            if sheet.is_some() || range.is_some() {
                return Err(AppError::Usage(
                    "--sheet と --range は xlsx 専用です".to_string(),
                ));
            }
            run_read_docx(file, para, out)
        }
    }
}

fn open_xlsx(file: &Path) -> Result<(OfficePackage, Vec<xlsx::SheetMeta>), AppError> {
    let mut package = OfficePackage::open(file, OfficeFormat::Xlsx)?;
    let workbook = package.read_part("xl/workbook.xml")?;
    let rels = xmlutil::parse_rels(&package.read_part("xl/_rels/workbook.xml.rels")?)?;
    let sheets = xlsx::parse_workbook(&workbook, &rels)?;
    Ok((package, sheets))
}

fn run_inspect_xlsx(file: &Path) -> Result<(), AppError> {
    let (mut package, sheets) = open_xlsx(file)?;
    let mut summaries = Vec::new();
    for sheet in &sheets {
        let xml = package.read_part(&sheet.path)?;
        let (rows, cols) = xlsx::sheet_dimension(&xml)
            .and_then(|d| xlsx::extent_from_dimension(&d))
            .unwrap_or(xlsx::scan_extent(&xml)?);
        summaries.push(SheetSummary {
            name: sheet.name.clone(),
            rows,
            cols,
        });
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&InspectOutput {
            file: file_label(file),
            format: "xlsx".to_string(),
            sheets: summaries,
        })
        .unwrap()
    );
    Ok(())
}

fn run_read_xlsx(
    file: &Path,
    sheet_filter: Option<&str>,
    range: Option<&str>,
    out: Option<&Path>,
) -> Result<(), AppError> {
    let filter = range
        .map(range::parse_range)
        .transpose()?
        .unwrap_or_else(RangeFilter::all);
    let (mut package, mut sheets) = open_xlsx(file)?;
    if let Some(name) = sheet_filter {
        sheets.retain(|sheet| sheet.name == name);
        if sheets.is_empty() {
            return Err(AppError::SheetNotFound(name.to_string()));
        }
    }
    let shared = package
        .read_part_opt("xl/sharedStrings.xml")?
        .map(|xml| xlsx::parse_shared_strings(&xml))
        .transpose()?
        .unwrap_or_default();
    let styles = package
        .read_part_opt("xl/styles.xml")?
        .map(|xml| xlsx::parse_styles(&xml))
        .transpose()?
        .unwrap_or_default();
    let extracted = package.extract_media(
        &out.map(Path::to_path_buf)
            .unwrap_or_else(|| default_out_dir(file)),
    )?;
    let mut dumps = Vec::new();
    let mut anchors = Vec::new();

    for sheet in &sheets {
        let xml = package.read_part(&sheet.path)?;
        let parsed = xlsx::parse_worksheet(&xml, &filter, &shared, &styles)?;
        if let Some(drawing_rid) = &parsed.drawing_rid
            && let Some(sheet_rels_xml) = package.read_part_opt(&rels_path_of(&sheet.path))?
        {
            let sheet_rels = xmlutil::parse_rels(&sheet_rels_xml)?;
            if let Some(target) = sheet_rels.get(drawing_rid) {
                let drawing_path = xmlutil::resolve_target(dirname(&sheet.path), target);
                let drawing_xml = package.read_part(&drawing_path)?;
                let drawing_rels =
                    xmlutil::parse_rels(&package.read_part(&rels_path_of(&drawing_path))?)?;
                for draft in media::parse_drawing(&drawing_xml)? {
                    anchors.push((sheet.name.clone(), draft, drawing_rels.clone()));
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
    println!(
        "{}",
        serde_json::to_string_pretty(&ReadOutput {
            file: file_label(file),
            format: "xlsx".to_string(),
            sheets: dumps,
            media: media::build_media_items(&extracted, anchors)?,
        })
        .unwrap()
    );
    Ok(())
}

fn run_inspect_docx(file: &Path) -> Result<(), AppError> {
    let mut package = OfficePackage::open(file, OfficeFormat::Docx)?;
    let styles = package
        .read_part_opt("word/styles.xml")?
        .map(|xml| docx::parse_styles(&xml))
        .transpose()?
        .unwrap_or_default();
    let document = docx::parse_document(&mut package, &styles, None)?;
    let body = &document.sections[0].blocks;
    println!(
        "{}",
        serde_json::to_string_pretty(&DocxInspectOutput {
            file: file_label(file),
            format: "docx".to_string(),
            sections: document
                .sections
                .iter()
                .map(|section| DocxSectionSummary {
                    section_type: section.section_type.clone(),
                    blocks: section.blocks.len(),
                })
                .collect(),
            outline: docx::collect_outline(body, &styles),
        })
        .unwrap()
    );
    Ok(())
}

fn run_read_docx(file: &Path, para: Option<&str>, out: Option<&Path>) -> Result<(), AppError> {
    let para_range = para.map(range::parse_block_range).transpose()?;
    let mut package = OfficePackage::open(file, OfficeFormat::Docx)?;
    let styles = package
        .read_part_opt("word/styles.xml")?
        .map(|xml| docx::parse_styles(&xml))
        .transpose()?
        .unwrap_or_default();
    let out_dir = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| default_out_dir(file));
    let extracted = package.extract_media(&out_dir)?;
    let document = docx::parse_document(&mut package, &styles, para_range)?;
    let rels = xmlutil::parse_rels(&package.read_part("word/_rels/document.xml.rels")?)?;
    let mut anchored = Vec::new();
    for drawing in document.drawings {
        let Some(rid) = drawing.info.embed_rid else {
            continue;
        };
        let Some(target) = rels.get(&rid) else {
            return Err(AppError::InvalidDocx(format!(
                "drawing のリレーション {rid} が見つかりません"
            )));
        };
        let part = xmlutil::resolve_target("word", target);
        let json_ref = format!("media/{}", basename(&part));
        let placement = if drawing.info.kind == "inline" {
            "inline"
        } else {
            "floating"
        };
        anchored.push((
            json_ref,
            MediaAnchor {
                sheet: None,
                from: None,
                to: None,
                pos: None,
                section: Some(drawing.section),
                block: Some(drawing.block),
                run: Some(drawing.run),
                pos_h: drawing.info.pos_h,
                pos_v: drawing.info.pos_v,
                anchor_type: drawing.info.kind,
                placement: placement.to_string(),
                name: drawing.info.name,
                ext: drawing.info.ext,
            },
        ));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&DocxReadOutput {
            file: file_label(file),
            format: "docx".to_string(),
            sections: document.sections,
            media: media::assemble_media_items(&extracted, anchored, OfficeFormat::Docx)?,
            unhandled: document.unhandled,
        })
        .unwrap()
    );
    Ok(())
}
