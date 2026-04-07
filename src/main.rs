mod compare;
mod display;

use anyhow::{Context, Result};
use can_dbc::Dbc;
use rfd::FileDialog;
use std::fs;
use std::path::PathBuf;

fn select_dbc_file(title: &str) -> Result<PathBuf> {
    FileDialog::new()
        .set_title(title)
        .add_filter("DBC files", &["dbc"])
        .add_filter("All files", &["*"])
        .pick_file()
        .context(format!("{}: ファイルが選択されませんでした", title))
}

fn select_save_file() -> Result<PathBuf> {
    FileDialog::new()
        .set_title("比較結果の保存先を選択")
        .add_filter("Text files", &["txt"])
        .add_filter("All files", &["*"])
        .set_file_name("dbc_comparison_result.txt")
        .save_file()
        .context("保存先が選択されませんでした")
}

fn read_dbc_string(path: &PathBuf) -> Result<String> {
    let bytes = fs::read(path)
        .with_context(|| format!("ファイルを読み込めません: {}", path.display()))?;

    // Try UTF-8 first, fall back to CP1252 (common encoding for DBC files)
    match String::from_utf8(bytes.clone()) {
        Ok(s) => Ok(s),
        Err(_) => {
            let (cow, _, had_errors) = encoding_rs::WINDOWS_1252.decode(&bytes);
            if had_errors {
                anyhow::bail!("ファイルのエンコーディングを判別できません: {}", path.display());
            }
            Ok(cow.into_owned())
        }
    }
}

fn parse_dbc(path: &PathBuf) -> Result<Dbc> {
    let content = read_dbc_string(path)?;
    let dbc = Dbc::try_from(content.as_str())
        .map_err(|e| anyhow::anyhow!("DBCパースエラー ({}): {:?}", path.display(), e))?;
    Ok(dbc)
}

fn main() -> Result<()> {
    println!("=== DBSee - DBC File Comparison Tool ===\n");

    println!("1つ目のDBCファイルを選択してください...");
    let path1 = select_dbc_file("1つ目のDBCファイルを選択")?;
    println!("  File 1: {}", path1.display());

    println!("2つ目のDBCファイルを選択してください...");
    let path2 = select_dbc_file("2つ目のDBCファイルを選択")?;
    println!("  File 2: {}\n", path2.display());

    let dbc1 = parse_dbc(&path1)?;
    let dbc2 = parse_dbc(&path2)?;

    let report = compare::compare_dbc(&dbc1, &dbc2);
    let output = display::format_report(&report, &path1, &path2);

    // Save to file with UTF-8 BOM for Windows compatibility
    println!("比較結果の保存先を選択してください...");
    let save_path = select_save_file()?;
    let mut bom_output = Vec::with_capacity(3 + output.len());
    bom_output.extend_from_slice(b"\xEF\xBB\xBF"); // UTF-8 BOM
    bom_output.extend_from_slice(output.as_bytes());
    fs::write(&save_path, &bom_output)
        .with_context(|| format!("ファイルの書き込みに失敗しました: {}", save_path.display()))?;

    println!("比較結果を保存しました: {}", save_path.display());

    Ok(())
}
