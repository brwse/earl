use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn top_level_help_lists_mcp_command() {
    let mut cmd = cargo_bin_cmd!("earl");
    cmd.arg("--help");

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("mcp"));
}

#[test]
fn mcp_help_includes_stdio_transport() {
    let mut cmd = cargo_bin_cmd!("earl");
    cmd.args(["mcp", "--help"]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("stdio"));
}

#[test]
fn mcp_help_includes_http_transport() {
    let mut cmd = cargo_bin_cmd!("earl");
    cmd.args(["mcp", "--help"]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("http"));
}

#[test]
fn mcp_help_includes_listen_flag() {
    let mut cmd = cargo_bin_cmd!("earl");
    cmd.args(["mcp", "--help"]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("--listen"));
}

#[test]
fn mcp_help_includes_mode_flag() {
    let mut cmd = cargo_bin_cmd!("earl");
    cmd.args(["mcp", "--help"]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("--mode"));
}

#[test]
fn mcp_help_includes_yes_flag() {
    let mut cmd = cargo_bin_cmd!("earl");
    cmd.args(["mcp", "--help"]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("--yes"));
}

#[test]
fn mcp_help_includes_discovery_subcommand() {
    let mut cmd = cargo_bin_cmd!("earl");
    cmd.args(["mcp", "--help"]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("discovery"));
}
