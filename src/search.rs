use crate::metadata::{self, DepOption};

#[derive(clap::Parser, Debug)]
pub struct SearchArgs {
    pub keyword: String,
}

pub async fn run_search(args: SearchArgs) -> Result<(), String> {
    let meta = metadata::fetch_metadata().await?;
    let query = args.keyword.to_lowercase();

    let mut results: Vec<(usize, &DepOption)> = Vec::new();

    for dep in &meta.all_deps {
        let key = dep.key.to_lowercase();
        let name = dep.text.to_lowercase();
        let desc = dep.description.to_lowercase();

        let mut score = 0;

        if key == query {
            score = 100;
        } else if name == query {
            score = 90;
        } else if key.contains(&query) {
            score = 80;
        } else if name.contains(&query) {
            score = 70;
        } else if desc.contains(&query) {
            score = 60;
        } else if is_fuzzy_match(&query, &key) || is_fuzzy_match(&query, &name) {
            score = 50;
        }

        if score > 0 {
            results.push((score, dep));
        }
    }

    if results.is_empty() {
        println!("No dependencies found matching '{}'", args.keyword);
        return Ok(());
    }

    // Sort by score descending
    results.sort_by(|a, b| b.0.cmp(&a.0));

    for (_, dep) in results {
        println!("{}", dep.key);
    }

    Ok(())
}

fn is_fuzzy_match(query: &str, target: &str) -> bool {
    let mut query_chars = query.chars();
    let mut current_char = query_chars.next();
    
    for c in target.chars() {
        if let Some(qc) = current_char {
            if c == qc {
                current_char = query_chars.next();
            }
        } else {
            return true;
        }
    }
    current_char.is_none()
}
