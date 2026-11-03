use std::env;
use std::fs;
use std::process;

// Deliberately straightforward (and allocation-heavy) CSV parser:
//  - the whole file is read into a String
//  - every field is copied into its own owned String
//  - every row becomes a Vec<String>, then an owned Record
//  - all records are kept in memory before being aggregated
//  - aggregation walks the records several times
#[derive(Debug)]
struct Record {
    id: u64,
    name: String,
    email: String,
    age: u32,
    salary: f64,
    city: String,
    active: bool,
    score: f64,
}

fn parse_line(line: &str) -> Result<Record, String> {
    let fields: Vec<String> = line.split(',').map(|s| s.trim().to_string()).collect();
    if fields.len() != 8 {
        return Err(format!("expected 8 fields, got {}: {}", fields.len(), line));
    }

    Ok(Record {
        id: fields[0].parse().map_err(|e| format!("bad id: {e}"))?,
        name: fields[1].clone(),
        email: fields[2].clone(),
        age: fields[3].parse().map_err(|e| format!("bad age: {e}"))?,
        salary: fields[4].parse().map_err(|e| format!("bad salary: {e}"))?,
        city: fields[5].clone(),
        active: fields[6].to_lowercase() == "true",
        score: fields[7].parse().map_err(|e| format!("bad score: {e}"))?,
    })
}

fn parse_csv(content: &str) -> Result<Vec<Record>, String> {
    let mut records = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if i == 0 || line.is_empty() {
            continue; // header / blank line
        }
        records.push(parse_line(line)?);
    }
    Ok(records)
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| "data.csv".to_string());

    let content = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("cannot read {path}: {e}");
        process::exit(1);
    });

    let records = parse_csv(&content).unwrap_or_else(|e| {
        eprintln!("parse error: {e}");
        process::exit(1);
    });

    let rows = records.len();
    let total_salary: f64 = records.iter().map(|r| r.salary).sum();
    let total_age: u64 = records.iter().map(|r| r.age as u64).sum();
    let active = records.iter().filter(|r| r.active).count();
    let total_score: f64 = records.iter().map(|r| r.score).sum();

    println!("rows         : {rows}");
    println!("total salary : {total_salary:.2}");
    println!("mean age     : {:.4}", total_age as f64 / rows as f64);
    println!("active       : {active}");
    println!("mean score   : {:.4}", total_score / rows as f64);

    // Touch the remaining fields so nothing is flagged as dead code.
    if let Some(last) = records.last() {
        println!(
            "last record  : #{} {} <{}> {}",
            last.id, last.name, last.email, last.city
        );
    }
}
