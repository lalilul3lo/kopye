// Integration testing can be done either by calling library functions directly or by invoking your CLI as a subprocess.
#[ignore]
#[test]
fn copy_template() {
    let repo = "gh:lalilul3lo/dev";
    let template = "rust";
    let destination = "baouncer";
    let mut cmd = assert_cmd::Command::cargo_bin("kopye").unwrap();

    cmd.arg("copy").arg(repo).arg(template).arg(destination);

    cmd.assert().success();
}

#[ignore]
#[test]
fn list_templates() {
    let repo = "gh:lalilul3lo/dev";
    let mut cmd = assert_cmd::Command::cargo_bin("kopye").unwrap();

    cmd.arg("list").arg(repo);

    cmd.assert()
        .success()
        .stdout(predicates::str::contains("hello world"));
}

// 1. Test that it creates all files found in blueprint whether or not it has a .tera extension.
// 2. Test that it removes .tera extension
// 3. Test that questions file does not get copied
// 4. test depends_on (ensure that it refers to an actual question)
// 5. test depends_on (ensure that the question is a boolean)
// 6. test transactions and rollback, give incomplete context (missing answer ) to tera to create
//    template
