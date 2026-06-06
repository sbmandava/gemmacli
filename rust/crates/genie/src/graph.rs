//! Correlation graph via lbug (LadybugDB / Kuzu): a
//! `(:File {path,sig,name})-[:Mentions {count}]->(:Entity {name})` graph.
//! M4 uses heuristic entity extraction (capitalized phrases + acronyms); LLM
//! extraction is a later enhancement.

use crate::config::Config;
use anyhow::{anyhow, Result};
use lbug::{Connection, Database, SystemConfig};
use std::collections::HashMap;
use std::path::Path;

fn open(cfg: &Config) -> Result<Database> {
    Database::new(&cfg.graph_db, SystemConfig::default()).map_err(|e| anyhow!("graph open: {e}"))
}

fn ensure_schema(conn: &Connection) -> Result<()> {
    for ddl in [
        "CREATE NODE TABLE IF NOT EXISTS File(path STRING, sig STRING, name STRING, PRIMARY KEY(path));",
        "CREATE NODE TABLE IF NOT EXISTS Entity(name STRING, PRIMARY KEY(name));",
        "CREATE REL TABLE IF NOT EXISTS Mentions(FROM File TO Entity, count INT64);",
    ] {
        conn.query(ddl).map_err(|e| anyhow!("schema: {e}"))?;
    }
    Ok(())
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

// ---------------------------------------------------------------------------
// Public ops (wired into the CLI / ingest)
// ---------------------------------------------------------------------------

/// Update the graph for one file's text (called during RAG ingest). Best-effort.
pub fn update_file_text(cfg: &Config, path: &str, sig: &str, text: &str) -> Result<()> {
    let db = open(cfg)?;
    let conn = Connection::new(&db).map_err(|e| anyhow!("graph conn: {e}"))?;
    ensure_schema(&conn)?;

    // Skip if this file+sig is already recorded.
    let mut existing: Option<String> = None;
    if let Ok(res) = conn.query(&format!("MATCH (f:File {{path:'{}'}}) RETURN f.sig;", esc(path))) {
        for row in res {
            existing = Some(format!("{}", row[0]));
        }
    }
    if existing.as_deref() == Some(sig) {
        return Ok(());
    }

    let name = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string());
    let entities = extract_entities(text, 40);

    conn.query(&format!(
        "MERGE (f:File {{path:'{}'}}) SET f.sig='{}', f.name='{}';",
        esc(path),
        esc(sig),
        esc(&name)
    ))
    .map_err(|e| anyhow!("upsert file: {e}"))?;
    let _ = conn.query(&format!(
        "MATCH (f:File {{path:'{}'}})-[m:Mentions]->() DELETE m;",
        esc(path)
    ));

    for (ent, count) in entities {
        let e = esc(&ent);
        let _ = conn.query(&format!("MERGE (e:Entity {{name:'{e}'}});"));
        let _ = conn.query(&format!(
            "MATCH (f:File {{path:'{}'}}), (e:Entity {{name:'{e}'}}) \
             MERGE (f)-[m:Mentions]->(e) SET m.count={count};",
            esc(path)
        ));
    }
    Ok(())
}

/// `--graph-stats`
pub fn stats(cfg: &Config) -> Result<()> {
    if !cfg.graph_db.exists() {
        println!("No correlation graph yet (index some files first).");
        return Ok(());
    }
    let db = open(cfg)?;
    let conn = Connection::new(&db).map_err(|e| anyhow!("graph conn: {e}"))?;
    ensure_schema(&conn)?;
    let n_files = scalar(&conn, "MATCH (f:File) RETURN count(f);");
    let n_ent = scalar(&conn, "MATCH (e:Entity) RETURN count(e);");
    let n_men = scalar(&conn, "MATCH (:File)-[m:Mentions]->(:Entity) RETURN count(m);");
    println!("Correlation graph: {}", cfg.graph_db.display());
    println!("  Files:    {n_files}");
    println!("  Entities: {n_ent}");
    println!("  Mentions: {n_men}");
    println!("  Top entities (by files mentioning):");
    if let Ok(res) = conn.query(
        "MATCH (f:File)-[:Mentions]->(e:Entity) RETURN e.name, count(f) AS c ORDER BY c DESC LIMIT 10;",
    ) {
        for row in res {
            println!("    {:>3}  {}", format!("{}", row[1]), row[0]);
        }
    }
    Ok(())
}

/// `--graph-query "<cypher>"`
pub fn query(cfg: &Config, cypher: &str) -> Result<()> {
    let db = open(cfg)?;
    let conn = Connection::new(&db).map_err(|e| anyhow!("graph conn: {e}"))?;
    ensure_schema(&conn)?;
    let res = conn.query(cypher).map_err(|e| anyhow!("query: {e}"))?;
    let cols = res.get_column_names();
    println!("{}", cols.join(" | "));
    for row in res {
        let cells: Vec<String> = row.iter().map(|v| format!("{v}")).collect();
        println!("{}", cells.join(" | "));
    }
    Ok(())
}

/// Build a graph-context string for a bare ask: which files mention the query's
/// entities, and what other entities co-occur. Returns None if nothing relevant.
pub fn correlate(cfg: &Config, question: &str) -> Option<String> {
    if !cfg.graph_db.exists() {
        return None;
    }
    let db = open(cfg).ok()?;
    let conn = Connection::new(&db).ok()?;
    ensure_schema(&conn).ok()?;
    let terms = extract_entities(question, 8);
    let mut out = String::new();
    for (term, _) in terms {
        let t = esc(&term);
        let mut files = Vec::new();
        if let Ok(res) = conn.query(&format!(
            "MATCH (e:Entity {{name:'{t}'}})<-[:Mentions]-(f:File) RETURN f.name LIMIT 20;"
        )) {
            for row in res {
                files.push(format!("{}", row[0]));
            }
        }
        let mut related = Vec::new();
        if let Ok(res) = conn.query(&format!(
            "MATCH (e:Entity {{name:'{t}'}})<-[:Mentions]-(:File)-[:Mentions]->(e2:Entity) \
             RETURN DISTINCT e2.name LIMIT 20;"
        )) {
            for row in res {
                related.push(format!("{}", row[0]));
            }
        }
        if !files.is_empty() || !related.is_empty() {
            out.push_str(&format!("- {term}: "));
            if !files.is_empty() {
                out.push_str(&format!("mentioned in [{}]", files.join(", ")));
            }
            if !related.is_empty() {
                out.push_str(&format!("; related to [{}]", related.join(", ")));
            }
            out.push('\n');
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn scalar(conn: &Connection, q: &str) -> String {
    if let Ok(res) = conn.query(q) {
        for row in res {
            return format!("{}", row[0]);
        }
    }
    "0".to_string()
}

// ---------------------------------------------------------------------------
// Heuristic entity extraction
// ---------------------------------------------------------------------------

/// Extract candidate entities: Title-Case phrases and ALL-CAPS acronyms,
/// ranked by frequency then length, capped at `max_entities`.
pub fn extract_entities(text: &str, max_entities: usize) -> Vec<(String, i64)> {
    let mut counts: HashMap<String, i64> = HashMap::new();
    for line in text.lines() {
        let words: Vec<&str> = line.split(|c: char| !(c.is_alphanumeric() || c == '&')).collect();
        let mut phrase: Vec<&str> = Vec::new();
        let flush = |phrase: &mut Vec<&str>, counts: &mut HashMap<String, i64>| {
            if !phrase.is_empty() {
                let p = phrase.join(" ");
                if p.len() >= 3 {
                    *counts.entry(p).or_insert(0) += 1;
                }
                phrase.clear();
            }
        };
        for w in words {
            if w.is_empty() {
                // A separator run (e.g. ". ") ends the current phrase.
                flush(&mut phrase, &mut counts);
                continue;
            }
            if is_title_case(w) {
                phrase.push(w);
            } else {
                flush(&mut phrase, &mut counts);
                if is_acronym(w) {
                    *counts.entry(w.to_string()).or_insert(0) += 1;
                }
            }
        }
        flush(&mut phrase, &mut counts);
    }
    // Drop trivial single common words.
    counts.retain(|k, _| !is_stopword(k));
    let mut v: Vec<(String, i64)> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.len().cmp(&a.0.len())));
    v.truncate(max_entities);
    v
}

fn is_title_case(w: &str) -> bool {
    let mut chars = w.chars();
    match chars.next() {
        Some(c) if c.is_uppercase() => chars.any(|c| c.is_lowercase()),
        _ => false,
    }
}

fn is_acronym(w: &str) -> bool {
    w.len() >= 2 && w.len() <= 10 && w.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && w.chars().any(|c| c.is_ascii_uppercase())
}

fn is_stopword(w: &str) -> bool {
    matches!(
        w,
        "The" | "This" | "That" | "These" | "Those" | "And" | "But" | "For" | "With" | "From"
            | "When" | "Where" | "What" | "Which" | "While" | "Their" | "There" | "Then" | "They"
    )
}
