
#[cfg(test)]
pub mod fixtures {
    #[macro_export]
    macro_rules! vec_of_strings {
        ($($x:expr),*) => (vec![$($x.to_string()),*]);
    }

    #[macro_export]
    macro_rules! vec_of_pathbuf {
        ($($x:expr),*) => (vec![$(PathBuf::from($x)),*]);
    }
}
