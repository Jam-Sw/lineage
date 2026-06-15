//! M0 proof: discover all in-scope repos for the authenticated user, clone/fetch
//! (reusing the cache), run author-filtered `git log --numstat`, and print the
//! lifetime diff both raw and with generated/vendored files excluded.
//!
//! Run:  GH_TOKEN=$(gh auth token) cargo run --example lineage_m0
//! Env:  MD_CACHE=<dir>  MD_EMAILS=a@x,b@y  to override the cache dir / identities.

use lineage_core::aggregate::{self, Rollup};
use lineage_core::engine::{self, git};
use lineage_core::github::GithubClient;
use lineage_core::numstat::{self, ChurnOptions};
use lineage_core::sensitive::Sensitive;
use lineage_core::types::Scope;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    if let Err(e) = run() {
        eprintln!("error [{}]: {}", e.code(), e);
        std::process::exit(1);
    }
}

fn run() -> lineage_core::Result<()> {
    let token = token();
    let cache = cache_dir();
    let client = GithubClient::new(token.expose().clone());

    let user = client.get_user()?;
    println!("authenticated as {} (id {})", user.login, user.id);

    let emails = emails(&user);
    println!("counting commits authored by: {}", emails.join(", "));

    let scope = Scope::default_for(&user.login);
    let repos = engine::discover(&client, &scope)?;
    println!(
        "discovered {} in-scope repos (forks excluded), cloning/fetching into {}\n",
        repos.len(),
        cache.display()
    );

    let started = Instant::now();
    let mut raw_churns = Vec::new();
    let mut filtered_churns = Vec::new();
    let total = repos.len();
    for (i, repo) in repos.iter().enumerate() {
        let dir = git::clone_or_fetch(&cache, repo, &token)?;
        let numstat_raw = git::numstat(&dir, &engine::authors_regex(&emails))?;
        let r = numstat::churn_for_repo(&repo.full_name, &numstat_raw, &ChurnOptions::raw());
        let f = numstat::churn_for_repo(&repo.full_name, &numstat_raw, &ChurnOptions::filtered());
        println!(
            "[{:>2}/{}] {:<48} raw +{:>7} -{:>7} | code +{:>6} -{:>6}",
            i + 1,
            total,
            truncate(&repo.full_name, 48),
            r.added,
            r.removed,
            f.added,
            f.removed
        );
        raw_churns.push(r);
        filtered_churns.push(f);
    }

    let raw = aggregate::rollup(&raw_churns);
    let filtered = aggregate::rollup(&filtered_churns);

    println!("\n================ LIFETIME Lineage ================");
    headline("RAW (everything git reports)", &raw);
    headline("CODE (generated/vendored excluded)  <- the real number", &filtered);

    println!("\n---- favourite languages (code only, top 15) ----");
    println!("  {:<22} {:>10} {:>10} {:>11}  share", "language", "added", "removed", "net");
    for l in filtered.languages.iter().take(15) {
        println!(
            "  {:<22} {:>10} {:>10} {:>+11} {:>5.1}%",
            l.language,
            commas(l.added as i64),
            commas(l.removed as i64),
            l.net,
            l.share * 100.0
        );
    }

    println!("\n---- top repos by code churn (top 15) ----");
    for repo in filtered.repos.iter().take(15) {
        println!(
            "  {:<48} +{:>7} -{:>7} net {:>+8}  {}",
            truncate(&repo.full_name, 48),
            repo.added,
            repo.removed,
            repo.net,
            repo.top_language.as_deref().unwrap_or("-")
        );
    }

    println!("\nprocessed {} repos in {:.1}s", total, started.elapsed().as_secs_f64());
    Ok(())
}

fn headline(label: &str, r: &Rollup) {
    println!(
        "\n{label}\n  added   {:>12}\n  removed {:>12}\n  NET     {:>+12}   ({} repos, {} languages)",
        commas(r.summary.added as i64),
        commas(r.summary.removed as i64),
        r.summary.net,
        r.summary.repo_count,
        r.summary.language_count
    );
}

fn token() -> Sensitive<String> {
    if let Ok(t) = std::env::var("GH_TOKEN") {
        if !t.trim().is_empty() {
            return Sensitive(t.trim().to_string());
        }
    }
    // Fallback to the gh CLI.
    let out = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .expect("set GH_TOKEN or install gh CLI");
    Sensitive(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn cache_dir() -> PathBuf {
    if let Ok(d) = std::env::var("MD_CACHE") {
        return PathBuf::from(d);
    }
    let home = std::env::var("HOME").expect("HOME");
    PathBuf::from(home).join(".cache/lineage/clones")
}

fn emails(user: &lineage_core::github::User) -> Vec<String> {
    if let Ok(list) = std::env::var("MD_EMAILS") {
        return list.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    }
    vec![user.noreply_email(), "jandrewmanson@gmail.com".to_string()]
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n - 1])
    }
}

fn commas(n: i64) -> String {
    let neg = n < 0;
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if neg {
        format!("-{out}")
    } else {
        out
    }
}
