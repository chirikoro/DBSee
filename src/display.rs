use crate::compare::{Diff, DiffReport, DiffStatus, MessageDiff, SectionDiff, SignalDiff};
use std::path::Path;

fn print_diff(diff: &Diff, indent: &str) {
    match diff {
        Diff::Added(detail) => println!("{}  Added: {}", indent, detail),
        Diff::Removed(detail) => println!("{}  Removed: {}", indent, detail),
        Diff::Changed { field, old, new } => {
            println!("{}  Changed: {}: {} -> {}", indent, field, old, new)
        }
    }
}

fn print_section(title: &str, diffs: &[Diff]) {
    if diffs.is_empty() {
        return;
    }
    println!("--- {} ---", title);
    for d in diffs {
        print_diff(d, "");
    }
    println!();
}

fn print_subsections(title: &str, sections: &[SectionDiff]) {
    if sections.is_empty() {
        return;
    }
    println!("--- {} ---", title);
    for sec in sections {
        println!("  {}:", sec.name);
        for d in &sec.diffs {
            print_diff(d, "  ");
        }
    }
    println!();
}

fn print_signal_diff(sd: &SignalDiff, indent: &str) {
    match sd.status {
        DiffStatus::Added => {
            println!("{}  Signal added: {}", indent, sd.name);
            for d in &sd.diffs {
                print_diff(d, &format!("{}  ", indent));
            }
        }
        DiffStatus::Removed => {
            println!("{}  Signal removed: {}", indent, sd.name);
            for d in &sd.diffs {
                print_diff(d, &format!("{}  ", indent));
            }
        }
        DiffStatus::Changed => {
            println!("{}  Signal {}:", indent, sd.name);
            for d in &sd.diffs {
                print_diff(d, &format!("{}  ", indent));
            }
        }
    }
}

fn print_message_diff(md: &MessageDiff) {
    let id_str = match md.id {
        can_dbc::MessageId::Standard(v) => format!("0x{:X}", v),
        can_dbc::MessageId::Extended(v) => format!("0x{:X}x", v),
    };

    match md.status {
        DiffStatus::Added => {
            println!("  Added: Message {} ({})", id_str, md.name);
        }
        DiffStatus::Removed => {
            println!("  Removed: Message {} ({})", id_str, md.name);
        }
        DiffStatus::Changed => {
            println!("  Message {} ({}):", id_str, md.name);
            for d in &md.diffs {
                print_diff(d, "  ");
            }
            for sd in &md.signal_diffs {
                print_signal_diff(sd, "    ");
            }
        }
    }
}

pub fn print_report(report: &DiffReport, path1: &Path, path2: &Path) {
    println!("=== DBC File Comparison ===");
    println!("File 1: {}", path1.display());
    println!("File 2: {}", path2.display());
    println!();

    if report.is_empty() {
        println!("No differences found. The files are identical.");
        return;
    }

    // Version
    print_section("Version", &report.version);

    // New Symbols
    print_section("New Symbols (NS_)", &report.new_symbols);

    // Nodes
    print_section("Nodes (BU_)", &report.nodes);

    // Messages & Signals
    if !report.messages.is_empty() {
        println!("--- Messages (BO_) & Signals (SG_) ---");
        for md in &report.messages {
            print_message_diff(md);
        }
        println!();
    }

    // Value Tables
    print_subsections("Value Tables (VAL_TABLE_)", &report.value_tables);

    // Comments
    print_section("Comments (CM_)", &report.comments);

    // Attribute Definitions
    print_section("Attribute Definitions (BA_DEF_)", &report.attribute_definitions);

    // Attribute Defaults
    print_section("Attribute Defaults (BA_DEF_DEF_)", &report.attribute_defaults);

    // Attribute Values
    print_section("Attribute Values (BA_)", &report.attribute_values);

    // Value Descriptions
    print_subsections("Value Descriptions (VAL_)", &report.value_descriptions);

    // Environment Variables
    print_section("Environment Variables (EV_)", &report.environment_variables);

    // Signal Type Refs
    print_section("Signal Type References (SIG_TYPE_REF_)", &report.signal_type_refs);

    // Signal Groups
    print_section("Signal Groups (SIG_GROUP_)", &report.signal_groups);

    // Signal Extended Value Types
    print_section(
        "Signal Extended Value Types (SIG_VALTYPE_)",
        &report.signal_extended_value_types,
    );

    // Extended Multiplex
    print_section("Extended Multiplex (SG_MUL_VAL_)", &report.extended_multiplex);

    // Summary
    println!("=== Summary ===");
    println!("Total differences: {}", report.total_count());
}
