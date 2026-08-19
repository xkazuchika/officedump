mod docx;
mod error;
mod ir;
mod mcp;
mod media;
mod output;
mod package;
mod pptx;
mod range;
mod xlsx;
mod xmlutil;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use serde::Serialize;

use error::{AppError, report};
use ir::{
    DocxInspectOutput, DocxReadOutput, DocxSectionSummary, InspectOutput, MediaAnchor,
    PptxInspectOutput, PptxReadOutput, ReadOutput, ReadSummary, SheetDump, SheetSummary,
    SlideTitle,
};
use output::{OutputPaths, render_read};
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
    /// 分解結果を content.json に書き出し、標準出力へ manifest を返す
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
        /// pptx のスライド範囲（例: 1:10）
        #[arg(long)]
        slide: Option<String>,
        /// 分解 JSON 全量を標準出力へ出す（content.json は生成しない）
        #[arg(long)]
        stdout: bool,
        /// メディア等の出力先（省略時は <ファイル名>.officedump/）
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// stdio トランスポートの MCP サーバーを起動する
    Mcp,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Inspect { file } => inspect_json(&file).map(|json| println!("{json}")),
        Command::Read {
            file,
            sheet,
            range,
            para,
            slide,
            stdout,
            out,
        } => read_json(
            &file,
            sheet.as_deref(),
            range.as_deref(),
            para.as_deref(),
            slide.as_deref(),
            stdout,
            out.as_deref(),
        )
        .map(|json| println!("{json}")),
        Command::Mcp => mcp::run_mcp(),
    };
    if let Err(e) = result {
        report(&e);
    }
}

fn detect_format(file: &Path) -> Result<OfficeFormat, AppError> {
    match file.extension().and_then(|e| e.to_str()) {
        Some("xlsx") => Ok(OfficeFormat::Xlsx),
        Some("docx") => Ok(OfficeFormat::Docx),
        Some("pptx") => Ok(OfficeFormat::Pptx),
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

fn to_json<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string_pretty(value).map_err(|e| AppError::Output(format!("JSON化に失敗: {e}")))
}

fn inspect_json(file: &Path) -> Result<String, AppError> {
    match detect_format(file)? {
        OfficeFormat::Xlsx => inspect_xlsx_json(file),
        OfficeFormat::Docx => inspect_docx_json(file),
        OfficeFormat::Pptx => inspect_pptx_json(file),
    }
}

fn read_json(
    file: &Path,
    sheet: Option<&str>,
    range: Option<&str>,
    para: Option<&str>,
    slide: Option<&str>,
    stdout: bool,
    out: Option<&Path>,
) -> Result<String, AppError> {
    match detect_format(file)? {
        OfficeFormat::Xlsx => {
            if para.is_some() || slide.is_some() {
                return Err(AppError::Usage(
                    "--para は docx 専用、--slide は pptx 専用です".to_string(),
                ));
            }
            read_xlsx_json(file, sheet, range, stdout, out)
        }
        OfficeFormat::Docx => {
            if sheet.is_some() || range.is_some() || slide.is_some() {
                return Err(AppError::Usage(
                    "--sheet と --range は xlsx 専用、--slide は pptx 専用です".to_string(),
                ));
            }
            read_docx_json(file, para, stdout, out)
        }
        OfficeFormat::Pptx => {
            if sheet.is_some() || range.is_some() || para.is_some() {
                return Err(AppError::Usage(
                    "--sheet/--range は xlsx 専用、--para は docx 専用です".to_string(),
                ));
            }
            read_pptx_json(file, slide, stdout, out)
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

fn inspect_xlsx_json(file: &Path) -> Result<String, AppError> {
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
    to_json(&InspectOutput {
        file: file_label(file),
        format: "xlsx".to_string(),
        sheets: summaries,
    })
}

fn read_xlsx_json(
    file: &Path,
    sheet_filter: Option<&str>,
    range: Option<&str>,
    stdout: bool,
    out: Option<&Path>,
) -> Result<String, AppError> {
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
    let styles_xml = package.read_part_opt("xl/styles.xml")?;
    let styles = styles_xml
        .as_ref()
        .map(|xml| xlsx::parse_styles(xml))
        .transpose()?
        .unwrap_or_default();
    let paths = OutputPaths::resolve(file, out)?;
    let extracted = package.extract_media(paths.root())?;
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
            rows: if parsed.rows.is_empty() {
                None
            } else {
                Some(parsed.rows)
            },
        });
    }
    let content = ReadOutput {
        file: file_label(file),
        format: "xlsx".to_string(),
        sheets: dumps,
        media: media::build_media_items(&extracted, anchors)?,
        styles: styles_xml.is_some().then(|| styles.clone()),
    };
    let summary = ReadSummary::Xlsx {
        sheets: content.sheets.len(),
        cells: content.sheets.iter().map(|sheet| sheet.cells.len()).sum(),
        media: content.media.len(),
    };
    render_read(
        &paths,
        stdout,
        content.file.clone(),
        content.format.clone(),
        summary,
        &content,
    )
}

fn inspect_docx_json(file: &Path) -> Result<String, AppError> {
    let mut package = OfficePackage::open(file, OfficeFormat::Docx)?;
    let styles = package
        .read_part_opt("word/styles.xml")?
        .map(|xml| docx::parse_styles(&xml))
        .transpose()?
        .unwrap_or_default();
    let document = docx::parse_document(&mut package, &styles, None)?;
    let body = &document.sections[0].blocks;
    to_json(&DocxInspectOutput {
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
}

fn read_docx_json(
    file: &Path,
    para: Option<&str>,
    stdout: bool,
    out: Option<&Path>,
) -> Result<String, AppError> {
    let para_range = para.map(range::parse_block_range).transpose()?;
    let mut package = OfficePackage::open(file, OfficeFormat::Docx)?;
    let styles = package
        .read_part_opt("word/styles.xml")?
        .map(|xml| docx::parse_styles(&xml))
        .transpose()?
        .unwrap_or_default();
    let paths = OutputPaths::resolve(file, out)?;
    let extracted = package.extract_media(paths.root())?;
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
                slide: None,
                z_order: None,
                geometry: None,
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
    let content = DocxReadOutput {
        file: file_label(file),
        format: "docx".to_string(),
        sections: document.sections,
        media: media::assemble_media_items(&extracted, anchored, OfficeFormat::Docx)?,
        unhandled: document.unhandled,
    };
    let summary = ReadSummary::Docx {
        sections: content.sections.len(),
        blocks: content
            .sections
            .iter()
            .map(|section| section.blocks.len())
            .sum(),
        media: content.media.len(),
    };
    render_read(
        &paths,
        stdout,
        content.file.clone(),
        content.format.clone(),
        summary,
        &content,
    )
}

fn open_pptx(file: &Path) -> Result<(OfficePackage, Vec<pptx::SlideMeta>), AppError> {
    let mut package = OfficePackage::open(file, OfficeFormat::Pptx)?;
    let presentation = package.read_part("ppt/presentation.xml")?;
    let rels = xmlutil::parse_rels(&package.read_part("ppt/_rels/presentation.xml.rels")?)?;
    let slides = pptx::parse_presentation(&presentation, &rels)?;
    Ok((package, slides))
}

fn inspect_pptx_json(file: &Path) -> Result<String, AppError> {
    let (mut package, slides) = open_pptx(file)?;
    let mut titles = Vec::new();
    for meta in &slides {
        let parsed = pptx::parse_slide(&package.read_part(&meta.path)?, meta.index)?;
        if let Some(title) = pptx::title(&parsed.slide) {
            titles.push(SlideTitle {
                index: meta.index,
                title,
            });
        }
    }
    to_json(&PptxInspectOutput {
        file: file_label(file),
        format: "pptx".to_string(),
        slides: slides.len(),
        titles,
    })
}

fn read_pptx_json(
    file: &Path,
    slide: Option<&str>,
    stdout: bool,
    out: Option<&Path>,
) -> Result<String, AppError> {
    let range = slide.map(range::parse_block_range).transpose()?;
    let (mut package, slides) = open_pptx(file)?;
    let paths = OutputPaths::resolve(file, out)?;
    let extracted = package.extract_media(paths.root())?;
    let mut output_slides = Vec::new();
    let mut anchored = Vec::new();

    for meta in slides {
        if let Some((from, to)) = range
            && !(from..=to).contains(&meta.index)
        {
            continue;
        }
        let parsed = pptx::parse_slide(&package.read_part(&meta.path)?, meta.index)?;
        let rels = package
            .read_part_opt(&rels_path_of(&meta.path))?
            .map(|xml| xmlutil::parse_rels(&xml))
            .transpose()?
            .unwrap_or_default();
        for media in parsed.media {
            let target = rels.get(&media.embed_rid).ok_or_else(|| {
                AppError::InvalidPptx(format!(
                    "スライド{}の画像リレーション {} が見つかりません",
                    meta.index, media.embed_rid
                ))
            })?;
            let part = xmlutil::resolve_target(dirname(&meta.path), target);
            let json_ref = format!("media/{}", basename(&part));
            anchored.push((
                json_ref,
                MediaAnchor {
                    sheet: None,
                    from: None,
                    to: None,
                    pos: None,
                    slide: Some(media.slide),
                    z_order: Some(media.z_order),
                    geometry: media.geometry,
                    section: None,
                    block: None,
                    run: None,
                    pos_h: None,
                    pos_v: None,
                    anchor_type: "picture".to_string(),
                    placement: "floating".to_string(),
                    name: media.name,
                    ext: None,
                },
            ));
        }
        output_slides.push(parsed.slide);
    }

    let content = PptxReadOutput {
        file: file_label(file),
        format: "pptx".to_string(),
        slides: output_slides,
        media: media::assemble_media_items(&extracted, anchored, OfficeFormat::Pptx)?,
    };
    let summary = ReadSummary::Pptx {
        slides: content.slides.len(),
        shapes: content.slides.iter().map(|slide| slide.shapes.len()).sum(),
        media: content.media.len(),
    };
    render_read(
        &paths,
        stdout,
        content.file.clone(),
        content.format.clone(),
        summary,
        &content,
    )
}
