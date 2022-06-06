use std::collections::HashSet;
use std::fs::OpenOptions;
use std::hash::Hash;
use std::io::Write;

pub fn has_unique_elements<T>(iter: T) -> bool
where
    T: IntoIterator,
    T::Item: Eq + Hash,
{
    let mut uniq = HashSet::new();
    iter.into_iter().all(move |x| uniq.insert(x))
}

pub fn debug_line_to_file(line: &str, filename: &str) {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(filename)
        .unwrap();
    match writeln!(file, "{}", line) {
        Ok(()) => (),
        Err(error) => println!("Failed to write to {} with error {:?}", filename, error),
    };
}
