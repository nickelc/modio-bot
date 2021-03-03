use git2::{Error, Repository};

fn main() -> Result<(), Error> {
    let repo = match Repository::discover(".") {
        Ok(repo) => repo,
        Err(_) => {
            println!("cargo:rustc-env=GIT_SHA=UNKNOWN");
            println!("cargo:rustc-env=GIT_SHA_SHORT=UNKNOWN");
            return Ok(());
        }
    };
    let repo_path = repo.path();

    let head = repo.find_reference("HEAD")?;
    let commit = head.peel_to_commit()?;
    let short_id = repo.revparse_single("HEAD")?.short_id()?;

    println!("cargo:rustc-env=GIT_SHA={}", commit.id().to_string());
    println!(
        "cargo:rustc-env=GIT_SHA_SHORT={}",
        short_id.as_str().unwrap_or_default()
    );
    println!(
        "cargo:rerun-if-changed={}",
        repo_path.join("HEAD").display()
    );

    if let Ok(resolved) = head.resolve() {
        if let Some(name) = resolved.name() {
            println!("cargo:rerun-if-changed={}", repo_path.join(name).display());
        }
    }
    Ok(())
}
