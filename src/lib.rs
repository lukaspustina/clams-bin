extern crate chrono;
extern crate clams;
#[macro_use]
extern crate clams_derive;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate handlebars;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;

#[cfg(test)]
extern crate spectral;

pub mod new_note {
    use chrono::prelude::*;
    use clams::config::prelude::*;
    use handlebars::Handlebars;
    use std::env;
    use std::fs::{self, File};
    use std::io::prelude::*;
    use std::process::Command;

    #[derive(Debug, Fail)]
    pub enum NnError {
        #[fail(display = "Could to parse publication date because {}", arg)]
        FailedToParsePublicationDate{ arg: String },
        #[fail(display = "Could to create new note because {}", arg)]
        FailedToCreateNewNote{ arg: String },
        #[fail(display = "Could not exec editor because {}", arg)]
        FailedToExecEditor{ arg: String },
        #[fail(display = "Could not render frontmatter template because {}", arg)]
        FailedToRenderFrontmatterTemplate{ arg: String },
        #[fail(display = "Could not write note file because {}", arg)]
        FailedToWriteNoteFile{ arg: String },
    }

    #[derive(Config, Debug, Serialize, Deserialize)]
    pub struct NewNoteConfig {
        pub notes_directory: String,
        pub notes_template: String,
    }

    pub fn title_to_file_name(title: &str) -> String {
        let mut res = title.to_lowercase().replace(" ", "-");
        res.push_str(".md");
        res
    }

    pub fn str_date_to_date(date: &str) -> Result<DateTime<Local>, NnError> {
        match date {
            "now" => Ok(Local::now()),
            _ => Local.datetime_from_str(date, "%Y-%m-%d %H:%M")
                    .map_err(|e| NnError::FailedToParsePublicationDate{ arg: e.to_string() }),
        }
    }

    pub fn date_to_iso_day(dt: &DateTime<Local> ) -> String {
        dt.format("%Y-%m-%d").to_string()
    }

    pub fn date_to_iso_time(dt: &DateTime<Local> ) -> String {
        dt.format("%Y-%m-%d %H:%M").to_string()
    }

    #[derive(Debug, Serialize)]
    pub struct FrontMatter {
        pub title: String,
        pub date: String,
    }

    pub fn create_note(path: &Path, template: &str, frontmatter: &FrontMatter) -> Result<(), NnError> {
        let content = render_template(template, frontmatter)?;
        let _ = write_content_to_file(&content, &path)?;

        Ok(())
    }

    pub fn render_template(template: &str, frontmatter: &FrontMatter) -> Result<String, NnError> {
        let mut handlebars = Handlebars::new();
        handlebars.register_template_string("frontmatter", template)
            .map_err(|e| NnError::FailedToRenderFrontmatterTemplate{ arg: e.to_string() })?;
        let text = handlebars.render("frontmatter", frontmatter)
            .map_err(|e| NnError::FailedToRenderFrontmatterTemplate{ arg: e.to_string() })?;

        Ok(text)
    }

    pub fn write_content_to_file(content: &str, path: &Path) -> Result<(), NnError> {
        // Make sure, the destnation dir exists.
        let dir = path.parent().ok_or_else(|| NnError::FailedToWriteNoteFile{ arg: "path does not contain directory".to_string() })?;
        if !dir.exists() {
            fs::create_dir(dir)
                .map_err(|e| NnError::FailedToWriteNoteFile{ arg: e.to_string() })?;
        }
        let mut file = File::create(path)
            .map_err(|e| NnError::FailedToWriteNoteFile{ arg: e.to_string() })?;
        file.write_all(content.as_bytes())
            .map_err(|e| NnError::FailedToWriteNoteFile{ arg: e.to_string() })?;

        Ok(())
    }

    pub fn open_editor(file: &Path) -> Result<(), NnError>{
        let editor = env::var_os("EDITOR").unwrap_or_else(|| "vi".to_string().into());

        let _ = Command::new(editor)
            .arg(file.as_os_str())
            .spawn()
            .map_err(|e| NnError::FailedToExecEditor { arg: e.to_string() })?;

        Ok(())
    }

    #[cfg(test)]
    mod test {
        pub use super::*;
        pub use spectral::prelude::*;

        #[test]
        fn title_to_file_name_okay() {
            let title = "This is just a Punk Rock song";
            let expected = "this-is-just-a-punk-rock-song.md".to_owned();

            let res = title_to_file_name(title);

            assert_that(&res).is_equal_to(expected);
        }

        #[test]
        fn str_date_to_date_okay() {
            let str_date = "2001-01-01 01:01";
            let expected = Local.ymd(2001, 01, 01).and_hms(01, 01, 00);

            let res = str_date_to_date(str_date);

            assert_that(&res).is_ok().is_equal_to(expected);
        }

        #[test]
        fn date_to_iso_day_okay() {
            let date = Local.ymd(2001, 01, 01).and_hms(01, 01, 00);
            let expected = "2001-01-01".to_owned();

            let res = date_to_iso_day(&date);

            assert_that(&res).is_equal_to(expected);
        }

        #[test]
        fn date_to_iso_time_okay() {
            let date = Local.ymd(2001, 01, 01).and_hms(01, 01, 00);
            let expected = "2001-01-01 01:01".to_owned();

            let res = date_to_iso_time(&date);

            assert_that(&res).is_equal_to(expected);
        }
    }
}

pub mod pelican_frontmatter {
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::{BufReader, Read, Write};
    use std::path::Path;

    #[derive(Debug, Fail)]
    pub enum ApfError {
        #[fail(display = "Could not open source file because {}", arg)]
        FailedToOpenSourceFile{ arg: String },
        #[fail(display = "Could not open destination file because {}", arg)]
        FailedToOpenDestinationFile{ arg: String },
        #[fail(display = "Failed to read because {}", arg)]
        FailedToRead{ arg: String },
        #[fail(display = "Failed to write because {}", arg)]
        FailedToWrite{ arg: String },
    }

    mod pelican {
        use super::ApfError;
        use std::collections::HashMap;

        #[derive(Debug, PartialEq)]
        pub struct FrontMatter {
            pub fields: HashMap<String, String>
        }

        pub fn parse_front_matter<T: AsRef<str>>(src: &[T]) -> Result<FrontMatter, ApfError> {
            let mut fields = HashMap::new();

            for line in src {
                let line = line.as_ref();
                let splits: Vec<_> = line.splitn(2, ":").collect();
                // We split once at max, so len==2 is _ to satisfy compiler for exhaustive matching.
                match splits.len() {
                    0 => {},
                    1 => { fields.insert(splits[0].to_owned(), "".to_owned()); },
                    _ => { fields.insert(splits[0].to_owned(), splits[1].trim().to_owned()); },
                }
            }

            Ok(FrontMatter{ fields })
        }

        #[cfg(test)]
        mod test {
            pub use super::*;
            pub use spectral::prelude::*;

            #[test]
            fn parse_front_matter_empty() {
                let front_matter = String::from("");
                let expected = FrontMatter{ fields: HashMap::new() };

                let front_matter: Vec<_> = front_matter.lines().collect();
                let res = parse_front_matter(front_matter.as_slice());

                //assert_that(&res).is_ok().is_equal_to(expected);
                assert_that(&res.is_ok()).is_true();
                assert_that(&res.unwrap()).is_equal_to(expected);
            }

            #[test]
            fn parse_front_matter_ok() {
                let front_matter = String::from(
r#"Title: With Proper TDD, You Get That
Date: 2012-07-27 12:00
Author: lukas
Category: Allgemein, Test Driving
Tags: TDD, Testing
Slug: with-proper-tdd-you-get-that
Status: published"#);
                let mut fields = HashMap::new();
                fields.insert("Title".to_owned(), "With Proper TDD, You Get That".to_owned());
                fields.insert("Date".to_owned(), "2012-07-27 12:00".to_owned());
                fields.insert("Author".to_owned(), "lukas".to_owned());
                fields.insert("Category".to_owned(), "Allgemein, Test Driving".to_owned());
                fields.insert("Tags".to_owned(), "TDD, Testing".to_owned());
                fields.insert("Slug".to_owned(), "with-proper-tdd-you-get-that".to_owned());
                fields.insert("Status".to_owned(), "published".to_owned());

                let expected = FrontMatter{ fields };

                let front_matter: Vec<_> = front_matter.lines().collect();
                let res = parse_front_matter(front_matter.as_slice());

                //assert_that(&res).is_ok().is_equal_to(expected);
                assert_that(&res.is_ok()).is_true();
                assert_that(&res.unwrap()).is_equal_to(expected);
            }
        }
    }

    #[derive(Debug, PartialEq)]
    pub enum FrontMatterType {
        Value(String),
        List(Vec<String>),
    }

    #[derive(Debug, PartialEq)]
    pub struct FrontMatter {
        pub fields: HashMap<String, FrontMatterType>
    }

    impl From<pelican::FrontMatter> for FrontMatter {
        fn from(pelican: pelican::FrontMatter) -> Self {
            let mut fields = HashMap::new();

            for (k, v) in pelican.fields {
                match k.to_lowercase().as_ref() {
                    "tags" => fields.insert("tags".to_owned(),
                        FrontMatterType::List(v.split(',').map(|s| s.trim().to_owned()).collect())),
                    "category" => fields.insert("categories".to_owned(),
                        FrontMatterType::List(v.split(',').map(|s| s.trim().to_owned()).collect())),
                    "slug" => None, // Remove this frontmatter field
                    key@_ => fields.insert(key.to_owned(),
                        FrontMatterType::Value(v.to_owned())),
                };
            }

            FrontMatter{ fields }
        }
    }

    impl FrontMatter {
        pub fn write(&self) -> String {
            let mut buf = String::new();

            buf.push_str("---\n");

            let mut keys: Vec<_> = self.fields.keys().collect();
            keys.sort();
            for k in keys {
                let line = match *self.fields.get(k).unwrap() { // Safe unwrap
                    FrontMatterType::Value(ref s) => format!("{}: \"{}\"\n", k, s),
                    FrontMatterType::List(ref l)  => {
                        let list: String = l.iter().map(|s| format!("- \"{}\"", s)).collect::<Vec<_>>().join("\n");
                        format!("{}:\n{}\n", k, list)
                    },
                };
                buf.push_str(&line);
            }

            buf.push_str("---\n");

            buf
        }
    } 

    pub fn adapt_pelican_frontmatter_in_file(src: &Path, dest: &Path) -> Result<(), ApfError> {
        let read = File::open(src).map_err(|e| ApfError::FailedToOpenSourceFile{ arg: e.to_string() })?;
        let mut write = File::create(dest).map_err(|e| ApfError::FailedToOpenDestinationFile{ arg: e.to_string() })?;

        adapt_pelican_frontmatter(read, &mut write)
    }

    /// Pelican's Frontmatter is really simple, but does not adhere the to front matter syntax used
    /// by Jekyll et al. The format is not yaml, but rather a sequence line separated key: value
    /// pairs until a blank.
    /// So let's keep this simple and read every line like a key value pair until the first blank
    /// line. The semantic has to be hardcoded for category and tags.
    fn adapt_pelican_frontmatter<R: Read, W: Write>(src: R, dest: &mut W) -> Result<(), ApfError> {
        let mut buf = String::new();
        let mut reader = BufReader::new(src);
        reader.read_to_string(&mut buf).map_err(|e| ApfError::FailedToRead{ arg: e.to_string() })?;
        let mut lines = buf.split('\n');

        let mut frontmatter_buf = Vec::new();
        loop {
            match lines.next() {
                Some(line) if line.is_empty() => break,
                Some(line) => { frontmatter_buf.push(line); },
                None => break,
            }
        }

        let pelican_frontmatter = pelican::parse_front_matter(frontmatter_buf.as_slice())?;
        let frontmatter: FrontMatter = pelican_frontmatter.into();

        dest.write(frontmatter.write().as_bytes())
            .map_err(|e| ApfError::FailedToWrite{ arg: e.to_string() })?;

        loop {
            match lines.next() {
                Some(line) => { 
                    dest.write(b"\n").map_err(|e| ApfError::FailedToWrite{ arg: e.to_string() })?;
                    dest.write(line.as_bytes()).map_err(|e| ApfError::FailedToWrite{ arg: e.to_string() })?;
                },
                None => break,
            }
        }

        Ok(())
    }

    #[cfg(test)]
    mod test {
        pub use super::*;
        pub use spectral::prelude::*;

        use std::io::BufWriter;

        mod adapt_pelican_frontmatter {
            use super::*;

            #[test]
            fn from_pelican_frontmatter() {
                let mut pelican_fields = HashMap::new();
                pelican_fields.insert("Title".to_owned(), "With Proper TDD, You Get That".to_owned());
                pelican_fields.insert("Date".to_owned(), "2012-07-27 12:00".to_owned());
                pelican_fields.insert("Author".to_owned(), "lukas".to_owned());
                pelican_fields.insert("Category".to_owned(), "Allgemein, Test Driving".to_owned());
                pelican_fields.insert("Tags".to_owned(), "TDD, Testing".to_owned());
                pelican_fields.insert("Slug".to_owned(), "with-proper-tdd-you-get-that".to_owned());
                pelican_fields.insert("Status".to_owned(), "published".to_owned());
                let pelican = pelican::FrontMatter{ fields: pelican_fields };

                let mut expected_fields = HashMap::new();
                expected_fields.insert("title".to_owned(), FrontMatterType::Value("With Proper TDD, You Get That".to_owned()));
                expected_fields.insert("date".to_owned(), FrontMatterType::Value("2012-07-27 12:00".to_owned()));
                expected_fields.insert("author".to_owned(), FrontMatterType::Value("lukas".to_owned()));
                expected_fields.insert("categories".to_owned(), FrontMatterType::List(vec!["Allgemein".to_owned(), "Test Driving".to_owned()]));
                expected_fields.insert("tags".to_owned(), FrontMatterType::List(vec!["TDD".to_owned(), "Testing".to_owned()]));
                expected_fields.insert("status".to_owned(), FrontMatterType::Value("published".to_owned()));
                let expected = FrontMatter{ fields: expected_fields };

                let frontmatter: FrontMatter = pelican.into();

                assert_that(&frontmatter).is_equal_to(expected);
            }

            #[test]
            fn write_frontmatter() {
                let mut fields = HashMap::new();
                fields.insert("title".to_owned(), FrontMatterType::Value("With Proper TDD, You Get That".to_owned()));
                fields.insert("date".to_owned(), FrontMatterType::Value("2012-07-27 12:00".to_owned()));
                fields.insert("author".to_owned(), FrontMatterType::Value("lukas".to_owned()));
                fields.insert("categories".to_owned(), FrontMatterType::List(vec!["Allgemein".to_owned(), "Test Driving".to_owned()]));
                fields.insert("tags".to_owned(), FrontMatterType::List(vec!["TDD".to_owned(), "Testing".to_owned()]));
                fields.insert("slug".to_owned(), FrontMatterType::Value("with-proper-tdd-you-get-that".to_owned()));
                fields.insert("status".to_owned(), FrontMatterType::Value("published".to_owned()));
                let frontmatter = FrontMatter{ fields };

                let expected = String::from(
r#"---
author: "lukas"
categories:
- "Allgemein"
- "Test Driving"
date: "2012-07-27 12:00"
slug: "with-proper-tdd-you-get-that"
status: "published"
tags:
- "TDD"
- "Testing"
title: "With Proper TDD, You Get That"
---
"#           );

                let res = frontmatter.write();

                assert_that(&res).is_equal_to(expected);
            }

            #[test]
            fn adapt_pelican_frontmatter_use_case_okay() {
                let src = String::from(
r#"Title: With Proper TDD, You Get That
Date: 2012-07-27 12:00
Author: lukas
Category: Allgemein, Test Driving
Tags: TDD, Testing
Slug: with-proper-tdd-you-get-that
Status: published

Dariusz Pasciak describes [how developing software without TDD is
like](http://blog.8thlight.com/dariusz-pasciak/2012/07/18/with-proper-tdd-you-get-that.html "With Proper TDD, You Get That")
and concludes:

End.
"#);
                let expected = String::from(
r#"---
author: "lukas"
categories:
- "Allgemein"
- "Test Driving"
date: "2012-07-27 12:00"
status: "published"
tags:
- "TDD"
- "Testing"
title: "With Proper TDD, You Get That"
---

Dariusz Pasciak describes [how developing software without TDD is
like](http://blog.8thlight.com/dariusz-pasciak/2012/07/18/with-proper-tdd-you-get-that.html "With Proper TDD, You Get That")
and concludes:

End.
"#);

                run_with_strings(&src, &expected);
            }

            fn run_with_strings(src: &String, expected: &String) -> () {
                let mut buffer = String::new();
                let res = {
                    let mut writer = BufWriter::new(
                        unsafe {
                            buffer.as_mut_vec()
                        }
                    );

                    adapt_pelican_frontmatter(src.as_bytes(), &mut writer)
                };

                assert_that(&res).is_ok();
                assert_that(&buffer).is_equal_to(expected);
            }

        }
    }
}

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
