#[derive(Debug)]
pub struct DocumentListing {
    pub id: String,
    pub authors: String,
    pub title: String,
    pub publisher: String,
    pub year_published: String,
    pub pages: String,
    pub language: String,
    pub file_size: String,
    pub extension: String,
    pub link: String,
}

impl DocumentListing {
    pub fn from(data: &Vec<String>) -> Self {
        if data.len() != 10 {
            // Somehow wrong format
            DocumentListing::new()
        } else {
            // Create with data
            let mut param_iter = data.iter();
            let iter = &mut param_iter;
            Self{
                id: next_processed(iter),
                authors: next_processed(iter),
                title: next_processed(iter),
                publisher: next_processed(iter),
                year_published: next_processed(iter),
                pages: next_processed(iter),
                language: next_processed(iter),
                file_size: next_processed(iter),
                extension: next_processed(iter),
                link: next_processed(iter)
            }
        }
    }

    pub fn new() -> Self {
        Self {
            id: ("".to_owned()),
            authors: ("".to_owned()),
            title: ("".to_owned()),
            publisher: ("".to_owned()),
            year_published: ("".to_owned()),
            pages: ("".to_owned()),
            language: ("".to_owned()),
            file_size: ("".to_owned()),
            extension: ("".to_owned()),
            link: ("".to_owned()),
        }
    }
}

impl std::fmt::Display for DocumentListing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "{} | {} | {} | {} pages | {} | {} | {}",
            self
                .title
                .chars()
                .filter(|c| c.is_alphabetic() || c == &' ')
                .collect::<String>()
                .trim(),
            self.authors,
            self.year_published,
            (if &self.pages == "" {
                "N/A"
            } else {
                &self.pages
            }),
            self.language,
            self.extension,
            self.file_size
        )
    }
}

fn next_processed<'a>(iter: &mut impl Iterator<Item = &'a String>) -> String{
    iter.next().unwrap_or(&"ERR".to_string()).to_string()
}
