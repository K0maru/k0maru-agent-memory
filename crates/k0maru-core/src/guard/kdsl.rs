use serde::{Deserialize, Serialize};

/// High-level decoded envelope of a K-DSL 0.1 machine-native packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdslEnvelope {
    pub version: String,
    pub domain: String,
    pub filepath: String,
    #[serde(default)]
    pub base_revision: String,
    #[serde(default)]
    pub coordinate: Option<KdslCoordinate>,
    #[serde(default)]
    pub operations: Vec<KdslOperation>,
    #[serde(default)]
    pub verification_invariants: Vec<String>,
}

/// AST coordinate identifying structural location in target file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdslCoordinate {
    pub node_type: String,
    pub identifier: String,
}

/// Atomic AST mutation operation within K-DSL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdslOperation {
    pub opcode: String,
    pub target: String,
    #[serde(default)]
    pub replacement: Option<String>,
}

/// Parses a K-DSL 0.1 input which can be either a raw wire string or JSON.
pub fn parse_kdsl(input: &str) -> Result<KdslEnvelope, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("K-DSL input is empty".to_string());
    }

    // 1. Try parsing as JSON
    if trimmed.starts_with('{') {
        return serde_json::from_str::<KdslEnvelope>(trimmed)
            .map_err(|e| format!("Failed to parse K-DSL JSON envelope: {}", e));
    }

    // 2. Parse raw wire grammar:
    // !δ:<version>|<domain>|<filepath>#<rev> §AST:<type>[<id>] @OP:<opcode> (<target>) [-> (<replacement>)] !Σ:VERIFY(<invariants>)
    let header_idx = trimmed
        .find("!δ:")
        .ok_or_else(|| "Missing required K-DSL header '!δ:'".to_string())?;

    let remainder = &trimmed[header_idx + 3..];

    // Split header from rest
    let mut parts = remainder.split_whitespace();
    let header_str = parts
        .next()
        .ok_or_else(|| "Malformed K-DSL header".to_string())?;

    let header_tokens: Vec<&str> = header_str.split('|').collect();
    if header_tokens.len() < 3 {
        return Err(format!(
            "Invalid K-DSL header structure '{}', expected '<version>|<domain>|<filepath>#<rev>'",
            header_str
        ));
    }

    let version = header_tokens[0].to_string();
    let domain = header_tokens[1].to_string();
    let file_and_rev = header_tokens[2];

    let (filepath, base_revision) = if let Some(hash_pos) = file_and_rev.find('#') {
        (
            file_and_rev[..hash_pos].to_string(),
            file_and_rev[hash_pos + 1..].to_string(),
        )
    } else {
        (file_and_rev.to_string(), String::new())
    };

    let rest = &remainder[header_str.len()..].trim();

    // Parse §AST coordinate if present
    let mut coordinate = None;
    let mut cursor = *rest;

    if let Some(ast_pos) = cursor.find("§AST:") {
        let after_ast = &cursor[ast_pos + 5..];
        let token_end = after_ast
            .find(|c: char| c.is_whitespace() || c == '@' || c == '!')
            .unwrap_or(after_ast.len());
        let coord_str = &after_ast[..token_end];

        if let Some(bracket_start) = coord_str.find('[') {
            if let Some(bracket_end) = coord_str.find(']') {
                let raw_type = &coord_str[..bracket_start];
                let node_type = match raw_type {
                    "F" => "Function",
                    "C" => "Class",
                    "M" => "Module",
                    "T" => "Type",
                    "V" => "Variable",
                    other => other,
                }
                .to_string();
                let identifier = coord_str[bracket_start + 1..bracket_end].to_string();
                coordinate = Some(KdslCoordinate {
                    node_type,
                    identifier,
                });
            }
        }
        cursor = after_ast[token_end..].trim();
    }

    // Parse @OP: operations
    let mut operations = Vec::new();
    let mut invariants = Vec::new();

    while let Some(op_pos) = cursor.find("@OP:") {
        let op_part = &cursor[op_pos + 4..];
        let next_section_pos = op_part
            .find("@OP:")
            .or_else(|| op_part.find("!Σ:"))
            .unwrap_or(op_part.len());

        let current_op_str = op_part[..next_section_pos].trim();
        cursor = op_part[next_section_pos..].trim();

        if let Some(op) = parse_single_op(current_op_str) {
            operations.push(op);
        }
    }

    // Parse !Σ:VERIFY(...) if present
    if let Some(verify_pos) = cursor.find("!Σ:VERIFY(") {
        let verify_part = &cursor[verify_pos + 10..];
        if let Some(close_pos) = verify_part.find(')') {
            let invs_str = &verify_part[..close_pos];
            for item in invs_str.split(',') {
                let t = item.trim();
                if !t.is_empty() {
                    invariants.push(t.to_string());
                }
            }
        }
    }

    Ok(KdslEnvelope {
        version,
        domain,
        filepath,
        base_revision,
        coordinate,
        operations,
        verification_invariants: invariants,
    })
}

fn parse_single_op(s: &str) -> Option<KdslOperation> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // Find opcode: first token before space or '('
    let opcode_end = s.find(|c: char| c.is_whitespace() || c == '(')?;
    let opcode = s[..opcode_end].trim().to_string();
    let rest = s[opcode_end..].trim();

    if let Some(arrow_pos) = rest.find("->") {
        let target_part = rest[..arrow_pos].trim();
        let repl_part = rest[arrow_pos + 2..].trim();

        let target = unwrap_parens(target_part).to_string();
        let replacement = Some(unwrap_parens(repl_part).to_string());

        Some(KdslOperation {
            opcode,
            target,
            replacement,
        })
    } else {
        let target = unwrap_parens(rest).to_string();
        Some(KdslOperation {
            opcode,
            target,
            replacement: None,
        })
    }
}

fn unwrap_parens(s: &str) -> &str {
    let t = s.trim();
    if t.starts_with('(') && t.ends_with(')') && t.len() >= 2 {
        t[1..t.len() - 1].trim()
    } else {
        t
    }
}
