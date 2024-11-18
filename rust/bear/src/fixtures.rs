// SPDX-License-Identifier: GPL-3.0-or-later

#[cfg(test)]
pub mod fixtures {
    #[macro_export]
    macro_rules! vec_of_strings {
        ($($x:expr),*) => (vec![$($x.to_string()),*]);
    }

    #[macro_export]
    macro_rules! map_of_strings {
        ($($k:expr => $v:expr),* $(,)?) => {{
            core::convert::From::from([$(($k.to_string(), $v.to_string()),)*])
        }};
    }

    #[macro_export]
    macro_rules! vec_of_pathbuf {
        ($($x:expr),*) => (vec![$(PathBuf::from($x)),*]);
    }
}
