use std::sync::Arc;

use dashmap::DashSet;

fn main() {
    let mut set: Arc<DashSet<String>> = Arc::new(DashSet::new());

    for entry in glob::glob("**/result.csv").expect("failed to read glob pattern") {
        match entry {
            Ok(path) => {
                set.insert(path.to_str().unwrap().to_string());
            }
            Err(e) => eprintln!("{:?}", e),
        };
    }

    println!("Unique path count: {}", set.len());

    for e in set.iter() {
        println!("{}", e.to_string());
    }
}
