// Integration testing can be done either by calling library functions directly or by invoking your CLI as a subprocess.
#[test]
fn copy_template() {
    let repo = "gh:lalilul3lo/dev";
    let template = "rust";
    let destination = "baouncer";
    let mut cmd = assert_cmd::Command::cargo_bin("kopye").unwrap();

    cmd.arg("copy").arg(repo).arg(template).arg(destination);

    cmd.assert().success();
}

#[test]
fn list_templates() {
    let repo = "gh:lalilul3lo/dev";
    let mut cmd = assert_cmd::Command::cargo_bin("kopye").unwrap();

    cmd.arg("list").arg(repo);

    cmd.assert()
        .success()
        .stdout(predicates::str::contains("hello world"));
}
