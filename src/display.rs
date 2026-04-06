use crate::compare::{Diff, DiffReport, DiffStatus, MessageDiff, SectionDiff, SignalDiff};
use std::fmt::Write;
use std::path::Path;

fn write_diff(out: &mut String, diff: &Diff, indent: &str) {
    match diff {
        Diff::Added(detail) => writeln!(out, "{}  Added: {}", indent, detail).unwrap(),
        Diff::Removed(detail) => writeln!(out, "{}  Removed: {}", indent, detail).unwrap(),
        Diff::Changed { field, old, new } => {
            writeln!(out, "{}  Changed: {}: {} -> {}", indent, field, old, new).unwrap()
        }
    }
}

fn write_section(out: &mut String, title: &str, diffs: &[Diff]) {
    if diffs.is_empty() {
        return;
    }
    writeln!(out, "--- {} ---", title).unwrap();
    for d in diffs {
        write_diff(out, d, "");
    }
    writeln!(out).unwrap();
}

fn write_subsections(out: &mut String, title: &str, sections: &[SectionDiff]) {
    if sections.is_empty() {
        return;
    }
    writeln!(out, "--- {} ---", title).unwrap();
    for sec in sections {
        writeln!(out, "  {}:", sec.name).unwrap();
        for d in &sec.diffs {
            write_diff(out, d, "  ");
        }
    }
    writeln!(out).unwrap();
}

fn write_signal_diff(out: &mut String, sd: &SignalDiff, indent: &str) {
    match sd.status {
        DiffStatus::Added => {
            writeln!(out, "{}  Signal added: {}", indent, sd.name).unwrap();
            for d in &sd.diffs {
                write_diff(out, d, &format!("{}  ", indent));
            }
        }
        DiffStatus::Removed => {
            writeln!(out, "{}  Signal removed: {}", indent, sd.name).unwrap();
            for d in &sd.diffs {
                write_diff(out, d, &format!("{}  ", indent));
            }
        }
        DiffStatus::Changed => {
            writeln!(out, "{}  Signal {}:", indent, sd.name).unwrap();
            for d in &sd.diffs {
                write_diff(out, d, &format!("{}  ", indent));
            }
        }
    }
}

fn write_message_diff(out: &mut String, md: &MessageDiff) {
    let id_str = match md.id {
        can_dbc::MessageId::Standard(v) => format!("0x{:X}", v),
        can_dbc::MessageId::Extended(v) => format!("0x{:X}x", v),
    };

    match md.status {
        DiffStatus::Added => {
            writeln!(out, "  Added: Message {} ({})", id_str, md.name).unwrap();
        }
        DiffStatus::Removed => {
            writeln!(out, "  Removed: Message {} ({})", id_str, md.name).unwrap();
        }
        DiffStatus::Changed => {
            writeln!(out, "  Message {} ({}):", id_str, md.name).unwrap();
            for d in &md.diffs {
                write_diff(out, d, "  ");
            }
            for sd in &md.signal_diffs {
                write_signal_diff(out, sd, "    ");
            }
        }
    }
}

pub fn format_report(report: &DiffReport, path1: &Path, path2: &Path) -> String {
    let mut out = String::new();

    writeln!(out, "=== DBC File Comparison ===").unwrap();
    writeln!(out, "File 1: {}", path1.display()).unwrap();
    writeln!(out, "File 2: {}", path2.display()).unwrap();
    writeln!(out).unwrap();

    if report.is_empty() {
        writeln!(out, "No differences found. The files are identical.").unwrap();
        return out;
    }

    write_section(&mut out, "Version", &report.version);
    write_section(&mut out, "New Symbols (NS_)", &report.new_symbols);
    write_section(&mut out, "Nodes (BU_)", &report.nodes);

    if !report.messages.is_empty() {
        writeln!(out, "--- Messages (BO_) & Signals (SG_) ---").unwrap();
        for md in &report.messages {
            write_message_diff(&mut out, md);
        }
        writeln!(out).unwrap();
    }

    write_subsections(&mut out, "Value Tables (VAL_TABLE_)", &report.value_tables);
    write_section(&mut out, "Comments (CM_)", &report.comments);
    write_section(&mut out, "Attribute Definitions (BA_DEF_)", &report.attribute_definitions);
    write_section(&mut out, "Attribute Defaults (BA_DEF_DEF_)", &report.attribute_defaults);
    write_section(&mut out, "Attribute Values (BA_)", &report.attribute_values);
    write_subsections(&mut out, "Value Descriptions (VAL_)", &report.value_descriptions);
    write_section(&mut out, "Environment Variables (EV_)", &report.environment_variables);
    write_section(&mut out, "Signal Type References (SIG_TYPE_REF_)", &report.signal_type_refs);
    write_section(&mut out, "Signal Groups (SIG_GROUP_)", &report.signal_groups);
    write_section(
        &mut out,
        "Signal Extended Value Types (SIG_VALTYPE_)",
        &report.signal_extended_value_types,
    );
    write_section(&mut out, "Extended Multiplex (SG_MUL_VAL_)", &report.extended_multiplex);

    writeln!(out, "=== Summary ===").unwrap();
    writeln!(out, "Total differences: {}", report.total_count()).unwrap();

    out
}
