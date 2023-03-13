#[cfg(test)]
use std::fs;
use std::env;
use std::process::{Command, Stdio};
use tempdir::TempDir;
use env_logger::{self, Target};
use log::info;

fn press_any() {
    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    write!(stdout, "Press any key to continue...").unwrap();
    stdout.flush().unwrap();
    use std::io::prelude::*;
    let _ = stdin.read(&mut [0u8]).unwrap();
}

fn get_env(var: &str) -> String {
    env::var(var).expect(format!("Error getting environment variable '{var}'").as_str())
}

fn do_invoke(cmd: &mut Command) {
    info!("cmd '{cmd:?}'");

    let mut child = cmd
            .stderr(Stdio::inherit())
            .spawn()
            .expect(format!("'{cmd:?}' command failed to start").as_str());

    if ! child.wait().unwrap().success() {
        if let Some(stderr) = child.stderr {
            panic!("stderr: {stderr:?}");
        }
        panic!("Failed '{cmd:?}'");
    }
}

#[test]
fn compare_with_sql_select() {
    env_logger::builder()
        .target(Target::Stderr)
        .init();

    let reporter_bin = "external_cfg";
    let reporter_bin_path = format!("target/debug/{reporter_bin}");
    let db_path = get_env("WSA_TEST_WINDOWS_DB_PATH");
    let cfg_path = get_env("WSA_TEST_CONFIGURATION_PATH");
    let sql_generator_path = get_env("WSA_TEST_SQL_GENERATOR_PATH");
    let sqlite3ext_h_path = get_env("ENV_SQLITE3EXT_H_PATH");
    let work_dir_name = format!("{reporter_bin}_testing");
    let work_dir = TempDir::new(work_dir_name.as_str()).expect("{work_dir_name} creation");
    let work_dir = work_dir.path().to_str().unwrap();

    info!("db_path: {db_path}");
    info!("cfg_path: {cfg_path}");
    info!("work_dir: {work_dir:?}");

    let mut cmd = Command::new(reporter_bin_path.as_str());
    let cmd = cmd
        .args(["--db-path", db_path.as_str()])
        .args(["--cfg-path", cfg_path.as_str()])
        .args(["--outdir", work_dir])
        .args(["--format", "csv"]);

    do_invoke(cmd);

    let files_to_copy = ["dtformat.c"];
    for file in files_to_copy {
        fs::copy(format!("tests/{file}"),
                 format!("{}/{file}", work_dir))
            .expect("copy file '{file}'");
    }

    let mut cmd =
        Command::new("clang");
    let cmd = cmd
        .current_dir(work_dir)
        .arg("-shared")
        .args(["-I", sqlite3ext_h_path.as_str()])
        .arg("-m32")
        .args(["-o", "dtformat.dll"])
        .arg("dtformat.c");

    do_invoke(cmd);

    let mut cmd =
        Command::new("python");
    let cmd = cmd
        .current_dir(work_dir)
        .arg(sql_generator_path)
        .arg(cfg_path);

    do_invoke(cmd);

    for item in std::path::Path::new(work_dir).read_dir().unwrap() {
        let path = item.unwrap().path();
        if let Some(extension) = path.extension() {
            if extension == "sql" {
                let mut cmd =
                    Command::new("sqlite3");
                let cmd = cmd
                    .current_dir(work_dir)
                    .arg(&db_path)
                    .arg(format!(".read {}", path.file_name().unwrap().to_str().unwrap()))
                    ;

                do_invoke(cmd);
            }
        }
    }


    press_any();

    fs::remove_dir_all(work_dir).unwrap();
}
