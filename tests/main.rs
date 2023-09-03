use std::path::Path;

fn exec(root: impl AsRef<Path>) -> anyhow::Result<()> {
    let bin = std::env!("CARGO_BIN_EXE_mtime-rewind");
    let status = std::process::Command::new(bin)
        .arg(root.as_ref())
        .spawn()?
        .wait()?;
    anyhow::ensure!(status.success());
    Ok(())
}

fn touch(path: &Path) -> anyhow::Result<()> {
    anyhow::ensure!(std::process::Command::new("touch")
        .arg(path)
        .spawn()?
        .wait()?
        .success());
    Ok(())
}
fn mtime(path: &Path) -> anyhow::Result<std::time::SystemTime> {
    Ok(std::fs::metadata(path)?.modified()?)
}

#[test]
fn test() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;

    let dir_path = dir.path();

    let a = dir_path.join("a");
    let b = dir_path.join("b");

    std::fs::write(&a, "a")?;
    std::fs::write(&b, "b")?;
    let mtime_a = mtime(&a)?;
    let mtime_b = mtime(&b)?;

    exec(&dir)?;
    assert_eq!(mtime_a, mtime(&a)?);
    assert_eq!(mtime_b, mtime(&b)?);

    touch(&a)?;
    std::fs::write(&b, "b2")?;
    let mtime_b2 = mtime(&b)?;

    exec(&dir)?;
    // a should be rewinded
    assert_eq!(mtime_a, mtime(&a)?);
    // but not b
    assert_ne!(mtime_b, mtime(&b)?);

    touch(&b)?;

    exec(&dir)?;
    // b should be rewinded
    assert_eq!(mtime_b2, mtime(&b)?);
    Ok(())
}
