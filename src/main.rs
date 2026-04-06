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
    display::print_report(&report, &path1, &path2);

    Ok(())
}
