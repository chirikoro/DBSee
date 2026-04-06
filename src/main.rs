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

fn parse_dbc(path: &PathBuf) -> Result<Dbc> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("ファイルを読み込めません: {}", path.display()))?;
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

    // Save to file
    println!("比較結果の保存先を選択してください...");
    let save_path = select_save_file()?;
    fs::write(&save_path, &output)
        .with_context(|| format!("ファイルの書き込みに失敗しました: {}", save_path.display()))?;

    println!("比較結果を保存しました: {}", save_path.display());

    Ok(())
}
