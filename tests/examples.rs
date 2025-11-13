use assert_cmd::Command;

fn run_example(example: &str) -> String {
    let mut cmd = Command::cargo_bin("mermaid-ascii").expect("binary exists");
    cmd.arg("--file")
        .arg(format!("examples/{}.mermaid", example));
    let output = cmd.assert().success().get_output().stdout.clone();
    String::from_utf8(output).expect("valid utf-8")
}

#[test]
fn basic_example_renders_nodes() {
    let output = run_example("basic");
    assert!(
        output.contains("A") && output.contains("D"),
        "basic example output:\n{}",
        output
    );
}

#[test]
fn labeled_edges_show_text() {
    let output = run_example("labels");
    assert!(
        output.contains("yes") && output.contains("retry"),
        "labels example output:\n{}",
        output
    );
}

#[test]
fn ascii_mode_respects_flag() {
    let mut cmd = Command::cargo_bin("mermaid-ascii").expect("binary exists");
    cmd.arg("--file")
        .arg("examples/basic.mermaid")
        .arg("--ascii");
    let output = cmd.assert().success().get_output().stdout.clone();
    let text = String::from_utf8(output).expect("valid utf-8");
    assert!(
        text.contains("-") && text.contains("|"),
        "ascii rendering should use plain characters:\n{}",
        text
    );
}
