extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate fern;
extern crate log;

#[cfg(test)]
extern crate spectral;

pub mod mv_files {
    use std::path::{Path, PathBuf};

    #[derive(Debug, Fail)]
    pub enum MvFilesError {
        #[fail(display = "Source directories missing")]
        EmptySources,
        #[fail(display = "Extensions missing")]
        EmptyExtensions,
        #[fail(display = "Invalid size arg '{}'", arg)]
        InvaildSize { arg: String },
        #[fail(display = "Invalid extensions list '{}'", arg)]
        InvalidExtensionsList { arg: String },
        #[fail(display = "Invalid file name'{}'", arg)]
        InvalidFileName { arg: String },
    }

    pub fn human_size_to_bytes(size: &str) -> Result<u64, MvFilesError> {
        if size.is_empty() {
            return Err(MvFilesError::InvaildSize {
                arg: String::from(size),
            });
        };

        let scales: &[_] = &['k', 'M', 'G', 'T', 'P'];
        let scale = size.chars().last().unwrap(); // safe because is_empty check
        let size = if scales.contains(&scale) {
            size.trim_right_matches(scales)
        } else {
            size
        };

        let size = size.parse::<u64>().map_err(|_| MvFilesError::InvaildSize {
            arg: String::from(size),
        })?;

        let size = match scale {
            'k' => size * 1024u64.pow(1),
            'M' => size * 1024u64.pow(2),
            'G' => size * 1024u64.pow(3),
            'T' => size * 1024u64.pow(4),
            'P' => size * 1024u64.pow(5),
            _ => size,
        };

        Ok(size)
    }

    pub fn destination_path<T: AsRef<Path>, S: AsRef<Path>>(
        destination_dir: T,
        file_path: S,
    ) -> Result<PathBuf, MvFilesError> {
        let file = file_path
            .as_ref()
            .file_name()
            .ok_or_else(|| MvFilesError::InvalidFileName {
                arg: format!("{:?}", file_path.as_ref()),
            })?;

        let mut path = PathBuf::new();
        path.push(destination_dir.as_ref());
        path.push(file);

        Ok(path)
    }

    pub fn parse_extensions(ext: &str) -> Result<Vec<&str>, MvFilesError> {
        if ext.is_empty() {
            return Err(MvFilesError::InvalidExtensionsList {
                arg: String::from(ext),
            });
        };

        let res: Vec<_> = ext.trim_right_matches(',').split(',').collect();

        Ok(res)
    }

    #[cfg(test)]
    mod test {
        pub use super::*;
        pub use spectral::prelude::*;

        mod human_size_to_bytes {
            use super::*;

            #[test]
            fn empty() {
                let res = human_size_to_bytes("");
                assert_that(&res).is_err();
            }

            #[test]
            fn nan() {
                let res = human_size_to_bytes("a10");
                assert_that(&res).is_err();
            }

            #[test]
            fn bytes() {
                assert_that(&human_size_to_bytes("100"))
                    .is_ok()
                    .is_equal_to(100)
            }

            #[test]
            fn kilo_bytes() {
                assert_that(&human_size_to_bytes("100k"))
                    .is_ok()
                    .is_equal_to(100 * 1024)
            }

            #[test]
            fn mega_bytes() {
                assert_that(&human_size_to_bytes("100M"))
                    .is_ok()
                    .is_equal_to(100 * 1024 * 1024)
            }

            #[test]
            fn giga_bytes() {
                assert_that(&human_size_to_bytes("100G"))
                    .is_ok()
                    .is_equal_to(100 * 1024 * 1024 * 1024)
            }

            #[test]
            fn tera_bytes() {
                assert_that(&human_size_to_bytes("100T"))
                    .is_ok()
                    .is_equal_to(100 * 1024 * 1024 * 1024 * 1024)
            }

            #[test]
            fn peta_bytes() {
                assert_that(&human_size_to_bytes("100P"))
                    .is_ok()
                    .is_equal_to(100 * 1024 * 1024 * 1024 * 1024 * 1024)
            }

            #[test]
            fn unknown_scale() {
                let res = human_size_to_bytes("100L");
                assert_that(&res).is_err();
            }
        }

        mod destination_path {
            use super::*;

            #[test]
            fn destination_path_ok() {
                let destination_dir = PathBuf::from("/tmp");
                let abs_file = PathBuf::from("/temp/a_file");
                let expected = PathBuf::from("/tmp/a_file");

                let res = destination_path(&destination_dir, &abs_file);

                assert_that(&res).is_ok().is_equal_to(expected);
            }
        }

        mod parse_extension {
            use super::*;

            #[test]
            fn empty() {
                let res = parse_extensions("");
                assert_that(&res).is_err();
            }

            #[test]
            fn one_extension() {
                let res = parse_extensions("mkv");
                assert_that(&res).is_ok().has_length(1);
            }

            #[test]
            fn two_extension() {
                let res = parse_extensions("mkv,avi");
                assert_that(&res).is_ok().has_length(2);
            }

            #[test]
            fn two_extension_trailing_sep() {
                let res = parse_extensions("mkv,avi,");
                assert_that(&res).is_ok().has_length(2);
            }
        }
    }
}
