use std::fs;
use std::io::Cursor;
use tempfile::tempdir;

use assert_cmd::Command;
use predicates::prelude::*;

use k0maru::offload::{inspect_node, OffloadEngine};

#[test]
fn test_offload_engine_short_logs_untruncated() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    let engine = OffloadEngine::new(&refs_dir).with_threshold(50);

    let input = (1..=10)
        .map(|i| format!("Line {}", i))
        .collect::<Vec<_>>()
        .join("\n");

    let result = engine
        .process_stream(Cursor::new(input.as_bytes()))
        .expect("Stream processing should succeed");

    assert!(!result.truncated, "Logs <= 50 lines must not be truncated");
    assert_eq!(result.total_lines, 10);
    assert!(result.log_path.is_none());
    assert!(result.mermaid_graph.is_empty());
    assert_eq!(result.summary_text, input);

    // refs directory should not have files created
    if refs_dir.exists() {
        let entries: Vec<_> = fs::read_dir(&refs_dir).unwrap().collect();
        assert_eq!(entries.len(), 0);
    }
}

#[test]
fn test_offload_engine_boundary_50_lines() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    let engine = OffloadEngine::new(&refs_dir).with_threshold(50);

    let input = (1..=50)
        .map(|i| format!("Line {}", i))
        .collect::<Vec<_>>()
        .join("\n");

    let result = engine
        .process_stream(Cursor::new(input.as_bytes()))
        .expect("Stream processing should succeed");

    assert!(!result.truncated);
    assert_eq!(result.total_lines, 50);
    assert!(result.log_path.is_none());
    assert!(result.mermaid_graph.is_empty());
}

#[test]
fn test_offload_engine_long_logs_truncated_and_persisted() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    let engine = OffloadEngine::new(&refs_dir).with_threshold(50);

    let input = (1..=55)
        .map(|i| format!("Trace log step {}", i))
        .collect::<Vec<_>>()
        .join("\n");

    let result = engine
        .process_stream(Cursor::new(input.as_bytes()))
        .expect("Stream processing should succeed");

    assert!(result.truncated, "Logs > 50 lines must be truncated");
    assert_eq!(result.total_lines, 55);
    assert!(result.node_id.starts_with("node_"));
    assert!(result.log_path.is_some());

    let saved_path = result.log_path.unwrap();
    assert!(saved_path.exists(), "Log file must be created on disk");

    let disk_content = fs::read_to_string(&saved_path).unwrap();
    assert!(disk_content.contains("Trace log step 1"));
    assert!(disk_content.contains("Trace log step 55"));

    // Verify Mermaid graph syntax
    assert!(
        result.mermaid_graph.contains("stateDiagram-v2"),
        "Mermaid graph must declare stateDiagram-v2"
    );
    assert!(
        result.mermaid_graph.contains(&result.node_id),
        "Mermaid graph must reference node_id"
    );
    assert!(
        result.mermaid_graph.contains("k0maru inspect"),
        "Mermaid graph must include inspect hint"
    );
    assert!(
        result.mermaid_graph.contains("55"),
        "Mermaid graph must include total lines"
    );

    // Verify summary text
    assert!(
        result.summary_text.contains(&result.node_id),
        "Summary text must contain node_id"
    );
    assert!(
        result.summary_text.contains("55"),
        "Summary text must contain total lines"
    );
}

#[test]
fn test_offload_engine_with_task_id() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    let engine = OffloadEngine::new(&refs_dir)
        .with_threshold(50)
        .with_task_id("cargo_build");

    let input = (1..=60)
        .map(|i| format!("Compiler warning line {}", i))
        .collect::<Vec<_>>()
        .join("\n");

    let result = engine
        .process_stream(Cursor::new(input.as_bytes()))
        .expect("Stream processing should succeed");

    assert!(result.truncated);
    let log_path = result.log_path.expect("log_path must exist");
    let file_name = log_path.file_name().unwrap().to_str().unwrap();

    assert!(
        file_name.starts_with("cargo_build_"),
        "File name must start with task_id prefix: {}",
        file_name
    );
    assert!(
        file_name.contains(&result.node_id),
        "File name must contain node_id: {}",
        file_name
    );
    assert!(
        result.mermaid_graph.contains("cargo_build"),
        "Mermaid graph should reflect task_id"
    );
}

#[test]
fn test_inspect_node_retrieval_and_variants() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    let engine = OffloadEngine::new(&refs_dir)
        .with_threshold(20)
        .with_task_id("test_job");

    let original_content = (1..=30)
        .map(|i| format!("Output row #{}", i))
        .collect::<Vec<_>>()
        .join("\n");

    let result = engine
        .process_stream(Cursor::new(original_content.as_bytes()))
        .unwrap();

    let node_id = result.node_id;

    // 1. Inspect using bare node_id
    let inspected1 = inspect_node(&refs_dir, &node_id).expect("Should find by node_id");
    assert!(inspected1.contains("Output row #1"));
    assert!(inspected1.contains("Output row #30"));

    // 2. Inspect using node_id.log
    let inspected2 =
        inspect_node(&refs_dir, &format!("{}.log", node_id)).expect("Should find with .log");
    assert_eq!(inspected1, inspected2);

    // 3. Inspect using full filename with task_id
    let full_name = format!("test_job_{}.log", node_id);
    let inspected3 = inspect_node(&refs_dir, &full_name).expect("Should find with full filename");
    assert_eq!(inspected1, inspected3);
}

#[test]
fn test_inspect_node_nonexistent_returns_error() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    fs::create_dir_all(&refs_dir).unwrap();

    let res = inspect_node(&refs_dir, "node_nonexistent_9999");
    assert!(res.is_err(), "Nonexistent node must return Err");
}

#[test]
fn test_offload_empty_input_graceful() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    let engine = OffloadEngine::new(&refs_dir);

    let result = engine
        .process_stream(Cursor::new(b""))
        .expect("Empty stream should succeed");

    assert!(!result.truncated);
    assert_eq!(result.total_lines, 0);
    assert!(result.log_path.is_none());
    assert!(result.mermaid_graph.is_empty());
    assert!(result.summary_text.is_empty());
}

#[test]
fn test_cli_offload_short_log_emitted_directly() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");

    let input = "hello world\nsecond line\nthird line\n";

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.args([
        "offload",
        "--threshold",
        "50",
        "--refs-dir",
        refs_dir.to_str().unwrap(),
    ])
    .write_stdin(input)
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "hello world\nsecond line\nthird line",
    ));
}

#[test]
fn test_cli_offload_long_log_and_inspect_e2e() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");

    let lines: Vec<String> = (1..=75)
        .map(|i| format!("Pipeline execution row {}", i))
        .collect();
    let input = lines.join("\n");

    // 1. Pipe long input to k0maru offload
    let mut offload_cmd = Command::cargo_bin("k0maru").unwrap();
    let output = offload_cmd
        .args([
            "offload",
            "--threshold",
            "50",
            "--task-id",
            "integration_task",
            "--refs-dir",
            refs_dir.to_str().unwrap(),
        ])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout_str = String::from_utf8_lossy(&output);
    assert!(
        stdout_str.contains("stateDiagram-v2"),
        "CLI stdout must include Mermaid stateDiagram-v2"
    );
    assert!(
        stdout_str.contains("node_"),
        "CLI stdout must include node_id"
    );
    assert!(
        stdout_str.contains("k0maru inspect"),
        "CLI stdout must include inspect guidance"
    );

    // Extract node_id from stdout
    let node_id = stdout_str
        .split_whitespace()
        .find(|w| w.starts_with("node_"))
        .expect("Must find node_id in stdout");

    // Clean any punctuation
    let clean_node_id = node_id.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');

    // 2. Inspect using k0maru inspect <NODE_ID>
    let mut inspect_cmd = Command::cargo_bin("k0maru").unwrap();
    inspect_cmd
        .args([
            "inspect",
            clean_node_id,
            "--refs-dir",
            refs_dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Pipeline execution row 1"))
        .stdout(predicate::str::contains("Pipeline execution row 75"));
}

#[test]
fn test_cli_inspect_missing_node_exits_code_2() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    fs::create_dir_all(&refs_dir).unwrap();

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.args([
        "inspect",
        "node_definitely_missing_12345",
        "--refs-dir",
        refs_dir.to_str().unwrap(),
    ])
    .assert()
    .failure()
    .code(2)
    .stderr(predicate::str::contains("node_definitely_missing_12345"));
}

#[test]
fn test_cli_offload_custom_threshold() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");

    // Input with 6 lines
    let input = "a\nb\nc\nd\ne\nf\n";

    // With threshold 5 -> should truncate
    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.args([
        "offload",
        "--threshold",
        "5",
        "--refs-dir",
        refs_dir.to_str().unwrap(),
    ])
    .write_stdin(input)
    .assert()
    .success()
    .stdout(predicate::str::contains("stateDiagram-v2"));

    // With threshold 10 -> should NOT truncate
    let mut cmd2 = Command::cargo_bin("k0maru").unwrap();
    cmd2.args([
        "offload",
        "--threshold",
        "10",
        "--refs-dir",
        refs_dir.to_str().unwrap(),
    ])
    .write_stdin(input)
    .assert()
    .success()
    .stdout(predicate::str::contains("a\nb\nc\nd\ne\nf"));
}
