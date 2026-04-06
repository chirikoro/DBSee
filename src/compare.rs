use can_dbc::{
    AttributeDefault, AttributeDefinition, AttributeValue, AttributeValueForObject,
    AttributeValuedForObjectType, ByteOrder, Comment, Dbc, EnvironmentVariable,
    ExtendedMultiplex, Message, MessageId, MultiplexIndicator, Signal, SignalExtendedValueType,
    SignalExtendedValueTypeList, SignalGroups, SignalTypeRef, Transmitter, ValDescription,
    ValueDescription, ValueTable, ValueType,
};
use std::collections::HashMap;

// ── Difference types ──

#[derive(Debug)]
pub enum Diff {
    Added(String),
    Removed(String),
    Changed { field: String, old: String, new: String },
}

#[derive(Debug)]
pub struct SignalDiff {
    pub name: String,
    pub status: DiffStatus,
    pub diffs: Vec<Diff>,
}

#[derive(Debug)]
pub struct MessageDiff {
    pub id: MessageId,
    pub name: String,
    pub status: DiffStatus,
    pub diffs: Vec<Diff>,
    pub signal_diffs: Vec<SignalDiff>,
}

#[derive(Debug, Clone, Copy)]
pub enum DiffStatus {
    Added,
    Removed,
    Changed,
}

#[derive(Debug)]
pub struct SectionDiff {
    pub name: String,
    pub diffs: Vec<Diff>,
}

#[derive(Debug)]
pub struct DiffReport {
    pub version: Vec<Diff>,
    pub nodes: Vec<Diff>,
    pub messages: Vec<MessageDiff>,
    pub value_tables: Vec<SectionDiff>,
    pub comments: Vec<Diff>,
    pub attribute_definitions: Vec<Diff>,
    pub attribute_defaults: Vec<Diff>,
    pub attribute_values: Vec<Diff>,
    pub value_descriptions: Vec<SectionDiff>,
    pub environment_variables: Vec<Diff>,
    pub signal_type_refs: Vec<Diff>,
    pub signal_groups: Vec<Diff>,
    pub signal_extended_value_types: Vec<Diff>,
    pub extended_multiplex: Vec<Diff>,
    pub new_symbols: Vec<Diff>,
}

impl DiffReport {
    pub fn total_count(&self) -> usize {
        let mut count = 0;
        count += self.version.len();
        count += self.nodes.len();
        count += self.new_symbols.len();
        for md in &self.messages {
            match md.status {
                DiffStatus::Added | DiffStatus::Removed => count += 1,
                DiffStatus::Changed => {
                    count += md.diffs.len();
                    for sd in &md.signal_diffs {
                        match sd.status {
                            DiffStatus::Added | DiffStatus::Removed => count += 1,
                            DiffStatus::Changed => count += sd.diffs.len(),
                        }
                    }
                }
            }
        }
        for vt in &self.value_tables {
            count += vt.diffs.len();
        }
        count += self.comments.len();
        count += self.attribute_definitions.len();
        count += self.attribute_defaults.len();
        count += self.attribute_values.len();
        for vd in &self.value_descriptions {
            count += vd.diffs.len();
        }
        count += self.environment_variables.len();
        count += self.signal_type_refs.len();
        count += self.signal_groups.len();
        count += self.signal_extended_value_types.len();
        count += self.extended_multiplex.len();
        count
    }

    pub fn is_empty(&self) -> bool {
        self.total_count() == 0
    }
}

// ── Helper formatters ──

fn fmt_message_id(id: &MessageId) -> String {
    match id {
        MessageId::Standard(v) => format!("0x{:X}", v),
        MessageId::Extended(v) => format!("0x{:X}x", v),
    }
}

fn fmt_transmitter(t: &Transmitter) -> String {
    match t {
        Transmitter::NodeName(n) => n.clone(),
        Transmitter::VectorXXX => "Vector__XXX".to_string(),
    }
}

fn fmt_byte_order(b: &ByteOrder) -> &str {
    match b {
        ByteOrder::LittleEndian => "LittleEndian",
        ByteOrder::BigEndian => "BigEndian",
    }
}

fn fmt_value_type(v: &ValueType) -> &str {
    match v {
        ValueType::Signed => "Signed",
        ValueType::Unsigned => "Unsigned",
    }
}

fn fmt_multiplex(m: &MultiplexIndicator) -> String {
    match m {
        MultiplexIndicator::Multiplexor => "Multiplexor".to_string(),
        MultiplexIndicator::MultiplexedSignal(n) => format!("Multiplexed(m{})", n),
        MultiplexIndicator::MultiplexorAndMultiplexedSignal(n) => {
            format!("MultiplexorAndMultiplexed(m{})", n)
        }
        MultiplexIndicator::Plain => "Plain".to_string(),
    }
}

fn fmt_attr_value(v: &AttributeValue) -> String {
    match v {
        AttributeValue::U64(n) => n.to_string(),
        AttributeValue::I64(n) => n.to_string(),
        AttributeValue::Double(n) => n.to_string(),
        AttributeValue::String(s) => format!("\"{}\"", s),
    }
}

fn fmt_attr_def(d: &AttributeDefinition) -> String {
    match d {
        AttributeDefinition::Message(s) => format!("Message(\"{}\")", s),
        AttributeDefinition::Node(s) => format!("Node(\"{}\")", s),
        AttributeDefinition::Signal(s) => format!("Signal(\"{}\")", s),
        AttributeDefinition::EnvironmentVariable(s) => format!("EnvVar(\"{}\")", s),
        AttributeDefinition::Plain(s) => format!("Plain(\"{}\")", s),
    }
}

fn attr_def_name(d: &AttributeDefinition) -> &str {
    match d {
        AttributeDefinition::Message(s)
        | AttributeDefinition::Node(s)
        | AttributeDefinition::Signal(s)
        | AttributeDefinition::EnvironmentVariable(s)
        | AttributeDefinition::Plain(s) => s,
    }
}

fn fmt_attr_obj(obj: &AttributeValuedForObjectType) -> String {
    match obj {
        AttributeValuedForObjectType::Raw(v) => format!("Raw({})", fmt_attr_value(v)),
        AttributeValuedForObjectType::NetworkNode(name, v) => {
            format!("Node \"{}\" = {}", name, fmt_attr_value(v))
        }
        AttributeValuedForObjectType::MessageDefinition(id, v) => {
            let val = v
                .as_ref()
                .map(|v| fmt_attr_value(v))
                .unwrap_or_else(|| "None".to_string());
            format!("Message {} = {}", fmt_message_id(id), val)
        }
        AttributeValuedForObjectType::Signal(msg_id, sig_name, v) => {
            format!(
                "Signal {} in {} = {}",
                sig_name,
                fmt_message_id(msg_id),
                fmt_attr_value(v)
            )
        }
        AttributeValuedForObjectType::EnvVariable(name, v) => {
            format!("EnvVar \"{}\" = {}", name, fmt_attr_value(v))
        }
    }
}

fn attr_value_key(obj: &AttributeValueForObject) -> String {
    let scope = match obj.value() {
        AttributeValuedForObjectType::Raw(_) => "raw".to_string(),
        AttributeValuedForObjectType::NetworkNode(name, _) => format!("node:{}", name),
        AttributeValuedForObjectType::MessageDefinition(id, _) => {
            format!("msg:{}", fmt_message_id(id))
        }
        AttributeValuedForObjectType::Signal(msg_id, sig, _) => {
            format!("sig:{}:{}", fmt_message_id(msg_id), sig)
        }
        AttributeValuedForObjectType::EnvVariable(name, _) => format!("env:{}", name),
    };
    format!("{}::{}", obj.name(), scope)
}

fn fmt_val_descriptions(descs: &[ValDescription]) -> String {
    descs
        .iter()
        .map(|d| format!("{} = \"{}\"", d.id(), d.description()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn fmt_extended_value_type(t: &SignalExtendedValueType) -> &str {
    match t {
        SignalExtendedValueType::SignedOrUnsignedInteger => "Integer",
        SignalExtendedValueType::IEEEfloat32Bit => "Float32",
        SignalExtendedValueType::IEEEdouble64bit => "Double64",
    }
}

// ── Comparison helpers ──

fn check_field<T: PartialEq + std::fmt::Display>(
    diffs: &mut Vec<Diff>,
    field: &str,
    old: &T,
    new: &T,
) {
    if old != new {
        diffs.push(Diff::Changed {
            field: field.to_string(),
            old: old.to_string(),
            new: new.to_string(),
        });
    }
}

fn check_field_str(diffs: &mut Vec<Diff>, field: &str, old: &str, new: &str) {
    if old != new {
        diffs.push(Diff::Changed {
            field: field.to_string(),
            old: old.to_string(),
            new: new.to_string(),
        });
    }
}

// ── Signal comparison ──

fn compare_signals(signals1: &[Signal], signals2: &[Signal]) -> Vec<SignalDiff> {
    let mut result = Vec::new();
    let map1: HashMap<&str, &Signal> = signals1.iter().map(|s| (s.name().as_str(), s)).collect();
    let map2: HashMap<&str, &Signal> = signals2.iter().map(|s| (s.name().as_str(), s)).collect();

    // Removed signals
    for (name, sig) in &map1 {
        if !map2.contains_key(name) {
            result.push(SignalDiff {
                name: name.to_string(),
                status: DiffStatus::Removed,
                diffs: vec![Diff::Removed(format!(
                    "start_bit={}, size={}, byte_order={}, value_type={}, factor={}, offset={}, min={}, max={}, unit=\"{}\"",
                    sig.start_bit, sig.size, fmt_byte_order(sig.byte_order()), fmt_value_type(sig.value_type()),
                    sig.factor, sig.offset, sig.min, sig.max, sig.unit()
                ))],
            });
        }
    }

    // Added signals
    for (name, sig) in &map2 {
        if !map1.contains_key(name) {
            result.push(SignalDiff {
                name: name.to_string(),
                status: DiffStatus::Added,
                diffs: vec![Diff::Added(format!(
                    "start_bit={}, size={}, byte_order={}, value_type={}, factor={}, offset={}, min={}, max={}, unit=\"{}\"",
                    sig.start_bit, sig.size, fmt_byte_order(sig.byte_order()), fmt_value_type(sig.value_type()),
                    sig.factor, sig.offset, sig.min, sig.max, sig.unit()
                ))],
            });
        }
    }

    // Changed signals
    for (name, s1) in &map1 {
        if let Some(s2) = map2.get(name) {
            let mut diffs = Vec::new();
            check_field(&mut diffs, "start_bit", &s1.start_bit, &s2.start_bit);
            check_field(&mut diffs, "size", &s1.size, &s2.size);
            check_field_str(
                &mut diffs,
                "byte_order",
                fmt_byte_order(s1.byte_order()),
                fmt_byte_order(s2.byte_order()),
            );
            check_field_str(
                &mut diffs,
                "value_type",
                fmt_value_type(s1.value_type()),
                fmt_value_type(s2.value_type()),
            );
            check_field(&mut diffs, "factor", &s1.factor, &s2.factor);
            check_field(&mut diffs, "offset", &s1.offset, &s2.offset);
            check_field(&mut diffs, "min", &s1.min, &s2.min);
            check_field(&mut diffs, "max", &s1.max, &s2.max);
            check_field_str(&mut diffs, "unit", s1.unit(), s2.unit());

            let mpx1 = fmt_multiplex(s1.multiplexer_indicator());
            let mpx2 = fmt_multiplex(s2.multiplexer_indicator());
            check_field_str(&mut diffs, "multiplexer_indicator", &mpx1, &mpx2);

            let mut recv1: Vec<&str> = s1.receivers().iter().map(|s| s.as_str()).collect();
            let mut recv2: Vec<&str> = s2.receivers().iter().map(|s| s.as_str()).collect();
            recv1.sort();
            recv2.sort();
            if recv1 != recv2 {
                diffs.push(Diff::Changed {
                    field: "receivers".to_string(),
                    old: format!("{:?}", recv1),
                    new: format!("{:?}", recv2),
                });
            }

            if !diffs.is_empty() {
                result.push(SignalDiff {
                    name: name.to_string(),
                    status: DiffStatus::Changed,
                    diffs,
                });
            }
        }
    }

    result
}

// ── Main comparison ──

pub fn compare_dbc(dbc1: &Dbc, dbc2: &Dbc) -> DiffReport {
    DiffReport {
        version: compare_version(dbc1, dbc2),
        nodes: compare_nodes(dbc1, dbc2),
        new_symbols: compare_new_symbols(dbc1, dbc2),
        messages: compare_messages(dbc1, dbc2),
        value_tables: compare_value_tables(dbc1, dbc2),
        comments: compare_comments(dbc1, dbc2),
        attribute_definitions: compare_attribute_definitions(dbc1, dbc2),
        attribute_defaults: compare_attribute_defaults(dbc1, dbc2),
        attribute_values: compare_attribute_values(dbc1, dbc2),
        value_descriptions: compare_value_descriptions(dbc1, dbc2),
        environment_variables: compare_environment_variables(dbc1, dbc2),
        signal_type_refs: compare_signal_type_refs(dbc1, dbc2),
        signal_groups: compare_signal_groups(dbc1, dbc2),
        signal_extended_value_types: compare_signal_extended_value_types(dbc1, dbc2),
        extended_multiplex: compare_extended_multiplex(dbc1, dbc2),
    }
}

fn compare_version(dbc1: &Dbc, dbc2: &Dbc) -> Vec<Diff> {
    let v1 = &dbc1.version().0;
    let v2 = &dbc2.version().0;
    if v1 != v2 {
        vec![Diff::Changed {
            field: "Version".to_string(),
            old: format!("\"{}\"", v1),
            new: format!("\"{}\"", v2),
        }]
    } else {
        vec![]
    }
}

fn compare_nodes(dbc1: &Dbc, dbc2: &Dbc) -> Vec<Diff> {
    let mut diffs = Vec::new();
    let names1: Vec<&str> = dbc1
        .nodes()
        .iter()
        .flat_map(|n| n.0.iter().map(|s| s.as_str()))
        .collect();
    let names2: Vec<&str> = dbc2
        .nodes()
        .iter()
        .flat_map(|n| n.0.iter().map(|s| s.as_str()))
        .collect();

    for name in &names1 {
        if !names2.contains(name) {
            diffs.push(Diff::Removed(name.to_string()));
        }
    }
    for name in &names2 {
        if !names1.contains(name) {
            diffs.push(Diff::Added(name.to_string()));
        }
    }
    diffs
}

fn compare_new_symbols(dbc1: &Dbc, dbc2: &Dbc) -> Vec<Diff> {
    let mut diffs = Vec::new();
    let syms1: Vec<&str> = dbc1.new_symbols().iter().map(|s| s.0.as_str()).collect();
    let syms2: Vec<&str> = dbc2.new_symbols().iter().map(|s| s.0.as_str()).collect();
    for s in &syms1 {
        if !syms2.contains(s) {
            diffs.push(Diff::Removed(s.to_string()));
        }
    }
    for s in &syms2 {
        if !syms1.contains(s) {
            diffs.push(Diff::Added(s.to_string()));
        }
    }
    diffs
}

fn compare_messages(dbc1: &Dbc, dbc2: &Dbc) -> Vec<MessageDiff> {
    let mut result = Vec::new();
    let map1: HashMap<u32, &Message> = dbc1.messages().iter().map(|m| (m.id().raw(), m)).collect();
    let map2: HashMap<u32, &Message> = dbc2.messages().iter().map(|m| (m.id().raw(), m)).collect();

    // Removed
    for (raw_id, msg) in &map1 {
        if !map2.contains_key(raw_id) {
            result.push(MessageDiff {
                id: *msg.id(),
                name: msg.name().clone(),
                status: DiffStatus::Removed,
                diffs: vec![],
                signal_diffs: vec![],
            });
        }
    }

    // Added
    for (raw_id, msg) in &map2 {
        if !map1.contains_key(raw_id) {
            result.push(MessageDiff {
                id: *msg.id(),
                name: msg.name().clone(),
                status: DiffStatus::Added,
                diffs: vec![],
                signal_diffs: vec![],
            });
        }
    }

    // Changed
    for (raw_id, m1) in &map1 {
        if let Some(m2) = map2.get(raw_id) {
            let mut diffs = Vec::new();
            check_field_str(&mut diffs, "name", m1.name(), m2.name());
            check_field(&mut diffs, "size", m1.size(), m2.size());
            let t1 = fmt_transmitter(m1.transmitter());
            let t2 = fmt_transmitter(m2.transmitter());
            check_field_str(&mut diffs, "transmitter", &t1, &t2);

            let signal_diffs = compare_signals(m1.signals(), m2.signals());

            if !diffs.is_empty() || !signal_diffs.is_empty() {
                result.push(MessageDiff {
                    id: *m1.id(),
                    name: m1.name().clone(),
                    status: DiffStatus::Changed,
                    diffs,
                    signal_diffs,
                });
            }
        }
    }

    // Sort by message ID for consistent output
    result.sort_by_key(|d| d.id.raw());
    result
}

fn compare_value_tables(dbc1: &Dbc, dbc2: &Dbc) -> Vec<SectionDiff> {
    let mut result = Vec::new();
    let map1: HashMap<&str, &ValueTable> =
        dbc1.value_tables().iter().map(|v| (v.name().as_str(), v)).collect();
    let map2: HashMap<&str, &ValueTable> =
        dbc2.value_tables().iter().map(|v| (v.name().as_str(), v)).collect();

    for (name, vt) in &map1 {
        if !map2.contains_key(name) {
            result.push(SectionDiff {
                name: name.to_string(),
                diffs: vec![Diff::Removed(format!(
                    "{}",
                    fmt_val_descriptions(vt.descriptions())
                ))],
            });
        }
    }
    for (name, vt) in &map2 {
        if !map1.contains_key(name) {
            result.push(SectionDiff {
                name: name.to_string(),
                diffs: vec![Diff::Added(format!(
                    "{}",
                    fmt_val_descriptions(vt.descriptions())
                ))],
            });
        }
    }
    for (name, vt1) in &map1 {
        if let Some(vt2) = map2.get(name) {
            if vt1.descriptions() != vt2.descriptions() {
                result.push(SectionDiff {
                    name: name.to_string(),
                    diffs: vec![Diff::Changed {
                        field: "values".to_string(),
                        old: fmt_val_descriptions(vt1.descriptions()),
                        new: fmt_val_descriptions(vt2.descriptions()),
                    }],
                });
            }
        }
    }
    result
}

fn comment_key(c: &Comment) -> String {
    match c {
        Comment::Node { name, .. } => format!("node:{}", name),
        Comment::Message { id, .. } => format!("msg:{}", fmt_message_id(id)),
        Comment::Signal {
            message_id, name, ..
        } => format!("sig:{}:{}", fmt_message_id(message_id), name),
        Comment::EnvVar { name, .. } => format!("env:{}", name),
        Comment::Plain { comment } => format!("plain:{}", comment),
    }
}

fn comment_text(c: &Comment) -> &str {
    match c {
        Comment::Node { comment, .. }
        | Comment::Message { comment, .. }
        | Comment::Signal { comment, .. }
        | Comment::EnvVar { comment, .. }
        | Comment::Plain { comment } => comment,
    }
}

fn comment_label(c: &Comment) -> String {
    match c {
        Comment::Node { name, .. } => format!("Node \"{}\"", name),
        Comment::Message { id, .. } => format!("Message {}", fmt_message_id(id)),
        Comment::Signal {
            message_id, name, ..
        } => format!("Signal \"{}\" in {}", name, fmt_message_id(message_id)),
        Comment::EnvVar { name, .. } => format!("EnvVar \"{}\"", name),
        Comment::Plain { .. } => "Plain".to_string(),
    }
}

fn compare_comments(dbc1: &Dbc, dbc2: &Dbc) -> Vec<Diff> {
    let mut diffs = Vec::new();
    let map1: HashMap<String, &Comment> = dbc1.comments().iter().map(|c| (comment_key(c), c)).collect();
    let map2: HashMap<String, &Comment> = dbc2.comments().iter().map(|c| (comment_key(c), c)).collect();

    for (key, c) in &map1 {
        if !map2.contains_key(key) {
            diffs.push(Diff::Removed(format!(
                "[{}] \"{}\"",
                comment_label(c),
                comment_text(c)
            )));
        }
    }
    for (key, c) in &map2 {
        if !map1.contains_key(key) {
            diffs.push(Diff::Added(format!(
                "[{}] \"{}\"",
                comment_label(c),
                comment_text(c)
            )));
        }
    }
    for (key, c1) in &map1 {
        if let Some(c2) = map2.get(key) {
            let t1 = comment_text(c1);
            let t2 = comment_text(c2);
            if t1 != t2 {
                diffs.push(Diff::Changed {
                    field: format!("[{}]", comment_label(c1)),
                    old: format!("\"{}\"", t1),
                    new: format!("\"{}\"", t2),
                });
            }
        }
    }
    diffs
}

fn compare_attribute_definitions(dbc1: &Dbc, dbc2: &Dbc) -> Vec<Diff> {
    let mut diffs = Vec::new();
    let map1: HashMap<&str, &AttributeDefinition> = dbc1
        .attribute_definitions()
        .iter()
        .map(|d| (attr_def_name(d), d))
        .collect();
    let map2: HashMap<&str, &AttributeDefinition> = dbc2
        .attribute_definitions()
        .iter()
        .map(|d| (attr_def_name(d), d))
        .collect();

    for (name, d) in &map1 {
        if !map2.contains_key(name) {
            diffs.push(Diff::Removed(fmt_attr_def(d)));
        }
    }
    for (name, d) in &map2 {
        if !map1.contains_key(name) {
            diffs.push(Diff::Added(fmt_attr_def(d)));
        }
    }
    for (name, d1) in &map1 {
        if let Some(d2) = map2.get(name) {
            if d1 != d2 {
                diffs.push(Diff::Changed {
                    field: name.to_string(),
                    old: fmt_attr_def(d1),
                    new: fmt_attr_def(d2),
                });
            }
        }
    }
    diffs
}

fn compare_attribute_defaults(dbc1: &Dbc, dbc2: &Dbc) -> Vec<Diff> {
    let mut diffs = Vec::new();
    let map1: HashMap<&str, &AttributeDefault> = dbc1
        .attribute_defaults()
        .iter()
        .map(|d| (d.name().as_str(), d))
        .collect();
    let map2: HashMap<&str, &AttributeDefault> = dbc2
        .attribute_defaults()
        .iter()
        .map(|d| (d.name().as_str(), d))
        .collect();

    for (name, d) in &map1 {
        if !map2.contains_key(name) {
            diffs.push(Diff::Removed(format!(
                "\"{}\" = {}",
                name,
                fmt_attr_value(d.value())
            )));
        }
    }
    for (name, d) in &map2 {
        if !map1.contains_key(name) {
            diffs.push(Diff::Added(format!(
                "\"{}\" = {}",
                name,
                fmt_attr_value(d.value())
            )));
        }
    }
    for (name, d1) in &map1 {
        if let Some(d2) = map2.get(name) {
            if d1.value() != d2.value() {
                diffs.push(Diff::Changed {
                    field: format!("\"{}\"", name),
                    old: fmt_attr_value(d1.value()),
                    new: fmt_attr_value(d2.value()),
                });
            }
        }
    }
    diffs
}

fn compare_attribute_values(dbc1: &Dbc, dbc2: &Dbc) -> Vec<Diff> {
    let mut diffs = Vec::new();
    let map1: HashMap<String, &AttributeValueForObject> = dbc1
        .attribute_values()
        .iter()
        .map(|a| (attr_value_key(a), a))
        .collect();
    let map2: HashMap<String, &AttributeValueForObject> = dbc2
        .attribute_values()
        .iter()
        .map(|a| (attr_value_key(a), a))
        .collect();

    for (key, a) in &map1 {
        if !map2.contains_key(key) {
            diffs.push(Diff::Removed(format!(
                "\"{}\" {}",
                a.name(),
                fmt_attr_obj(a.value())
            )));
        }
    }
    for (key, a) in &map2 {
        if !map1.contains_key(key) {
            diffs.push(Diff::Added(format!(
                "\"{}\" {}",
                a.name(),
                fmt_attr_obj(a.value())
            )));
        }
    }
    for (key, a1) in &map1 {
        if let Some(a2) = map2.get(key) {
            if a1.value() != a2.value() {
                diffs.push(Diff::Changed {
                    field: format!("\"{}\"", a1.name()),
                    old: fmt_attr_obj(a1.value()),
                    new: fmt_attr_obj(a2.value()),
                });
            }
        }
    }
    diffs
}

fn value_desc_key(vd: &ValueDescription) -> String {
    match vd {
        ValueDescription::Signal {
            message_id, name, ..
        } => format!("sig:{}:{}", fmt_message_id(message_id), name),
        ValueDescription::EnvironmentVariable { name, .. } => format!("env:{}", name),
    }
}

fn value_desc_label(vd: &ValueDescription) -> String {
    match vd {
        ValueDescription::Signal {
            message_id, name, ..
        } => format!("Signal \"{}\" in {}", name, fmt_message_id(message_id)),
        ValueDescription::EnvironmentVariable { name, .. } => format!("EnvVar \"{}\"", name),
    }
}

fn value_desc_values(vd: &ValueDescription) -> &[ValDescription] {
    match vd {
        ValueDescription::Signal {
            value_descriptions, ..
        } => value_descriptions,
        ValueDescription::EnvironmentVariable {
            value_descriptions, ..
        } => value_descriptions,
    }
}

fn compare_value_descriptions(dbc1: &Dbc, dbc2: &Dbc) -> Vec<SectionDiff> {
    let mut result = Vec::new();
    let map1: HashMap<String, &ValueDescription> = dbc1
        .value_descriptions()
        .iter()
        .map(|v| (value_desc_key(v), v))
        .collect();
    let map2: HashMap<String, &ValueDescription> = dbc2
        .value_descriptions()
        .iter()
        .map(|v| (value_desc_key(v), v))
        .collect();

    for (key, vd) in &map1 {
        if !map2.contains_key(key) {
            result.push(SectionDiff {
                name: value_desc_label(vd),
                diffs: vec![Diff::Removed(fmt_val_descriptions(value_desc_values(vd)))],
            });
        }
    }
    for (key, vd) in &map2 {
        if !map1.contains_key(key) {
            result.push(SectionDiff {
                name: value_desc_label(vd),
                diffs: vec![Diff::Added(fmt_val_descriptions(value_desc_values(vd)))],
            });
        }
    }
    for (key, vd1) in &map1 {
        if let Some(vd2) = map2.get(key) {
            let vals1 = value_desc_values(vd1);
            let vals2 = value_desc_values(vd2);
            if vals1 != vals2 {
                // Compare individual value descriptions
                let vmap1: HashMap<String, &str> = vals1
                    .iter()
                    .map(|d| (format!("{}", d.id()), d.description().as_str()))
                    .collect();
                let vmap2: HashMap<String, &str> = vals2
                    .iter()
                    .map(|d| (format!("{}", d.id()), d.description().as_str()))
                    .collect();

                let mut diffs = Vec::new();
                for (id, desc) in &vmap1 {
                    if !vmap2.contains_key(id) {
                        diffs.push(Diff::Removed(format!("{} = \"{}\"", id, desc)));
                    }
                }
                for (id, desc) in &vmap2 {
                    if !vmap1.contains_key(id) {
                        diffs.push(Diff::Added(format!("{} = \"{}\"", id, desc)));
                    }
                }
                for (id, desc1) in &vmap1 {
                    if let Some(desc2) = vmap2.get(id) {
                        if desc1 != desc2 {
                            diffs.push(Diff::Changed {
                                field: id.clone(),
                                old: format!("\"{}\"", desc1),
                                new: format!("\"{}\"", desc2),
                            });
                        }
                    }
                }
                if !diffs.is_empty() {
                    result.push(SectionDiff {
                        name: value_desc_label(vd1),
                        diffs,
                    });
                }
            }
        }
    }
    result
}

fn compare_environment_variables(dbc1: &Dbc, dbc2: &Dbc) -> Vec<Diff> {
    let mut diffs = Vec::new();
    let map1: HashMap<&str, &EnvironmentVariable> = dbc1
        .environment_variables()
        .iter()
        .map(|e| (e.name().as_str(), e))
        .collect();
    let map2: HashMap<&str, &EnvironmentVariable> = dbc2
        .environment_variables()
        .iter()
        .map(|e| (e.name().as_str(), e))
        .collect();

    for (name, _) in &map1 {
        if !map2.contains_key(name) {
            diffs.push(Diff::Removed(name.to_string()));
        }
    }
    for (name, _) in &map2 {
        if !map1.contains_key(name) {
            diffs.push(Diff::Added(name.to_string()));
        }
    }
    for (name, e1) in &map1 {
        if let Some(e2) = map2.get(name) {
            let mut field_diffs = Vec::new();
            if e1.typ() != e2.typ() {
                field_diffs.push(Diff::Changed {
                    field: format!("\"{}\".type", name),
                    old: format!("{:?}", e1.typ()),
                    new: format!("{:?}", e2.typ()),
                });
            }
            check_field(&mut field_diffs, &format!("\"{}\".min", name), e1.min(), e2.min());
            check_field(&mut field_diffs, &format!("\"{}\".max", name), e1.max(), e2.max());
            check_field_str(
                &mut field_diffs,
                &format!("\"{}\".unit", name),
                e1.unit(),
                e2.unit(),
            );
            check_field(
                &mut field_diffs,
                &format!("\"{}\".initial_value", name),
                e1.initial_value(),
                e2.initial_value(),
            );
            check_field(
                &mut field_diffs,
                &format!("\"{}\".ev_id", name),
                e1.ev_id(),
                e2.ev_id(),
            );
            if e1.access_type() != e2.access_type() {
                field_diffs.push(Diff::Changed {
                    field: format!("\"{}\".access_type", name),
                    old: format!("{:?}", e1.access_type()),
                    new: format!("{:?}", e2.access_type()),
                });
            }
            if e1.access_nodes() != e2.access_nodes() {
                field_diffs.push(Diff::Changed {
                    field: format!("\"{}\".access_nodes", name),
                    old: format!("{:?}", e1.access_nodes()),
                    new: format!("{:?}", e2.access_nodes()),
                });
            }
            diffs.extend(field_diffs);
        }
    }
    diffs
}

fn compare_signal_type_refs(dbc1: &Dbc, dbc2: &Dbc) -> Vec<Diff> {
    let mut diffs = Vec::new();
    let key = |r: &SignalTypeRef| {
        format!(
            "{}:{}",
            fmt_message_id(r.message_id()),
            r.signal_name()
        )
    };
    let map1: HashMap<String, &SignalTypeRef> =
        dbc1.signal_type_refs().iter().map(|r| (key(r), r)).collect();
    let map2: HashMap<String, &SignalTypeRef> =
        dbc2.signal_type_refs().iter().map(|r| (key(r), r)).collect();

    for (k, r) in &map1 {
        if !map2.contains_key(k) {
            diffs.push(Diff::Removed(format!(
                "{} {} -> {}",
                fmt_message_id(r.message_id()),
                r.signal_name(),
                r.signal_type_name()
            )));
        }
    }
    for (k, r) in &map2 {
        if !map1.contains_key(k) {
            diffs.push(Diff::Added(format!(
                "{} {} -> {}",
                fmt_message_id(r.message_id()),
                r.signal_name(),
                r.signal_type_name()
            )));
        }
    }
    for (k, r1) in &map1 {
        if let Some(r2) = map2.get(k) {
            if r1.signal_type_name() != r2.signal_type_name() {
                diffs.push(Diff::Changed {
                    field: format!(
                        "{} {}",
                        fmt_message_id(r1.message_id()),
                        r1.signal_name()
                    ),
                    old: r1.signal_type_name().clone(),
                    new: r2.signal_type_name().clone(),
                });
            }
        }
    }
    diffs
}

fn compare_signal_groups(dbc1: &Dbc, dbc2: &Dbc) -> Vec<Diff> {
    let mut diffs = Vec::new();
    let key =
        |g: &SignalGroups| format!("{}:{}", fmt_message_id(g.message_id()), g.name());
    let map1: HashMap<String, &SignalGroups> =
        dbc1.signal_groups().iter().map(|g| (key(g), g)).collect();
    let map2: HashMap<String, &SignalGroups> =
        dbc2.signal_groups().iter().map(|g| (key(g), g)).collect();

    for (k, g) in &map1 {
        if !map2.contains_key(k) {
            diffs.push(Diff::Removed(format!(
                "{} \"{}\" signals={:?}",
                fmt_message_id(g.message_id()),
                g.name(),
                g.signal_names()
            )));
        }
    }
    for (k, g) in &map2 {
        if !map1.contains_key(k) {
            diffs.push(Diff::Added(format!(
                "{} \"{}\" signals={:?}",
                fmt_message_id(g.message_id()),
                g.name(),
                g.signal_names()
            )));
        }
    }
    for (k, g1) in &map1 {
        if let Some(g2) = map2.get(k) {
            let mut field_diffs = Vec::new();
            if g1.repetitions() != g2.repetitions() {
                field_diffs.push(Diff::Changed {
                    field: format!("\"{}\" repetitions", g1.name()),
                    old: g1.repetitions().to_string(),
                    new: g2.repetitions().to_string(),
                });
            }
            if g1.signal_names() != g2.signal_names() {
                field_diffs.push(Diff::Changed {
                    field: format!("\"{}\" signal_names", g1.name()),
                    old: format!("{:?}", g1.signal_names()),
                    new: format!("{:?}", g2.signal_names()),
                });
            }
            diffs.extend(field_diffs);
        }
    }
    diffs
}

fn compare_signal_extended_value_types(dbc1: &Dbc, dbc2: &Dbc) -> Vec<Diff> {
    let mut diffs = Vec::new();
    let key = |e: &SignalExtendedValueTypeList| {
        format!("{}:{}", fmt_message_id(e.message_id()), e.signal_name())
    };
    let map1: HashMap<String, &SignalExtendedValueTypeList> = dbc1
        .signal_extended_value_type_list()
        .iter()
        .map(|e| (key(e), e))
        .collect();
    let map2: HashMap<String, &SignalExtendedValueTypeList> = dbc2
        .signal_extended_value_type_list()
        .iter()
        .map(|e| (key(e), e))
        .collect();

    for (k, e) in &map1 {
        if !map2.contains_key(k) {
            diffs.push(Diff::Removed(format!(
                "{} {} = {}",
                fmt_message_id(e.message_id()),
                e.signal_name(),
                fmt_extended_value_type(e.signal_extended_value_type())
            )));
        }
    }
    for (k, e) in &map2 {
        if !map1.contains_key(k) {
            diffs.push(Diff::Added(format!(
                "{} {} = {}",
                fmt_message_id(e.message_id()),
                e.signal_name(),
                fmt_extended_value_type(e.signal_extended_value_type())
            )));
        }
    }
    for (k, e1) in &map1 {
        if let Some(e2) = map2.get(k) {
            if e1.signal_extended_value_type() != e2.signal_extended_value_type() {
                diffs.push(Diff::Changed {
                    field: format!(
                        "{} {}",
                        fmt_message_id(e1.message_id()),
                        e1.signal_name()
                    ),
                    old: fmt_extended_value_type(e1.signal_extended_value_type()).to_string(),
                    new: fmt_extended_value_type(e2.signal_extended_value_type()).to_string(),
                });
            }
        }
    }
    diffs
}

fn compare_extended_multiplex(dbc1: &Dbc, dbc2: &Dbc) -> Vec<Diff> {
    let mut diffs = Vec::new();
    let key = |e: &ExtendedMultiplex| {
        format!(
            "{}:{}:{}",
            fmt_message_id(e.message_id()),
            e.signal_name(),
            e.multiplexor_signal_name()
        )
    };
    let map1: HashMap<String, &ExtendedMultiplex> =
        dbc1.extended_multiplex().iter().map(|e| (key(e), e)).collect();
    let map2: HashMap<String, &ExtendedMultiplex> =
        dbc2.extended_multiplex().iter().map(|e| (key(e), e)).collect();

    for (k, e) in &map1 {
        if !map2.contains_key(k) {
            diffs.push(Diff::Removed(format!(
                "{} {} mux_by {} mappings={:?}",
                fmt_message_id(e.message_id()),
                e.signal_name(),
                e.multiplexor_signal_name(),
                e.mappings()
            )));
        }
    }
    for (k, e) in &map2 {
        if !map1.contains_key(k) {
            diffs.push(Diff::Added(format!(
                "{} {} mux_by {} mappings={:?}",
                fmt_message_id(e.message_id()),
                e.signal_name(),
                e.multiplexor_signal_name(),
                e.mappings()
            )));
        }
    }
    for (k, e1) in &map1 {
        if let Some(e2) = map2.get(k) {
            if e1.mappings() != e2.mappings() {
                diffs.push(Diff::Changed {
                    field: format!(
                        "{} {} mux_by {}",
                        fmt_message_id(e1.message_id()),
                        e1.signal_name(),
                        e1.multiplexor_signal_name()
                    ),
                    old: format!("{:?}", e1.mappings()),
                    new: format!("{:?}", e2.mappings()),
                });
            }
        }
    }
    diffs
}
