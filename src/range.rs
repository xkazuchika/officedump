use crate::error::AppError;

/// セル範囲フィルタ。`None` はその軸に制限がないことを示す。
#[derive(Debug, Clone, Default)]
pub struct RangeFilter {
    pub min_col: Option<u32>,
    pub max_col: Option<u32>,
    pub min_row: Option<u32>,
    pub max_row: Option<u32>,
}

impl RangeFilter {
    pub fn all() -> Self {
        Self::default()
    }

    pub fn contains(&self, col: u32, row: u32) -> bool {
        if let Some(min) = self.min_col
            && col < min
        {
            return false;
        }
        if let Some(max) = self.max_col
            && col > max
        {
            return false;
        }
        if let Some(min) = self.min_row
            && row < min
        {
            return false;
        }
        if let Some(max) = self.max_row
            && row > max
        {
            return false;
        }
        true
    }
}

/// "A" -> 1, "Z" -> 26, "AA" -> 27
pub fn col_to_num(letters: &str) -> Result<u32, AppError> {
    let mut n: u32 = 0;
    for ch in letters.chars() {
        if !ch.is_ascii_alphabetic() {
            return Err(AppError::InvalidRange(format!(
                "列指定が不正です: {letters}"
            )));
        }
        n = n * 26 + (ch.to_ascii_uppercase() as u32 - 'A' as u32 + 1);
    }
    if n == 0 {
        return Err(AppError::InvalidRange("列指定が空です".to_string()));
    }
    Ok(n)
}

/// 1 -> "A", 27 -> "AA"
pub fn num_to_col(mut n: u32) -> String {
    let mut s = Vec::new();
    while n > 0 {
        n -= 1;
        s.push((b'A' + (n % 26) as u8) as char);
        n /= 26;
    }
    s.iter().rev().collect()
}

/// "B2" -> (2, 2)  （列, 行）
pub fn parse_cell_ref(s: &str) -> Result<(u32, u32), AppError> {
    let split = s
        .find(|c: char| c.is_ascii_digit())
        .ok_or_else(|| AppError::InvalidRange(format!("セル参照が不正です: {s}")))?;
    let (col, row) = s.split_at(split);
    let row: u32 = row
        .parse()
        .map_err(|_| AppError::InvalidRange(format!("セル参照が不正です: {s}")))?;
    Ok((col_to_num(col)?, row))
}

/// 範囲の片側: 行番号 / 列文字 / セル参照のいずれかを (col, row) にする。
fn half(p: &str) -> Result<(Option<u32>, Option<u32>), AppError> {
    if p.is_empty() {
        return Err(AppError::InvalidRange("範囲の端点が空です".to_string()));
    }
    if p.chars().all(|c| c.is_ascii_digit()) {
        let row: u32 = p
            .parse()
            .map_err(|_| AppError::InvalidRange(format!("行番号が不正です: {p}")))?;
        Ok((None, Some(row)))
    } else if p.chars().all(|c| c.is_ascii_alphabetic()) {
        Ok((Some(col_to_num(p)?), None))
    } else {
        let (c, r) = parse_cell_ref(p)?;
        Ok((Some(c), Some(r)))
    }
}

/// "A1:F50" / "A:C" / "1:30" / "B2" をパースする。
pub fn parse_range(s: &str) -> Result<RangeFilter, AppError> {
    if let Some((a, b)) = s.split_once(':') {
        let (a_col, a_row) = half(a)?;
        let (b_col, b_row) = half(b)?;
        Ok(RangeFilter {
            min_col: a_col,
            max_col: b_col,
            min_row: a_row,
            max_row: b_row,
        })
    } else {
        let (col, row) = half(s)?;
        Ok(RangeFilter {
            min_col: col,
            max_col: col,
            min_row: row,
            max_row: row,
        })
    }
}

/// "1:10" -> (1, 10)。ブロック範囲（docx の --para 用）。
pub fn parse_block_range(s: &str) -> Result<(u32, u32), AppError> {
    let (a, b) = s.split_once(':').ok_or_else(|| {
        AppError::InvalidRange(format!("ブロック範囲は N:M 形式で指定してください: {s}"))
    })?;
    let from: u32 = a
        .parse()
        .map_err(|_| AppError::InvalidRange(format!("ブロック範囲が不正です: {s}")))?;
    let to: u32 = b
        .parse()
        .map_err(|_| AppError::InvalidRange(format!("ブロック範囲が不正です: {s}")))?;
    if from == 0 || to < from {
        return Err(AppError::InvalidRange(format!(
            "ブロック範囲が不正です（1始まり、from <= to）: {s}"
        )));
    }
    Ok((from, to))
}
