#[derive(Debug)]
pub struct DocumentListing {
    pub id: String,
    pub authors: String,
    pub title: String,
    pub publisher: String,
    pub year_published: u16,
    pub pages: String,
    pub language: String,
    pub file_size: String,
    pub extension: String,
}

impl DocumentListing {
    pub fn from(data: Vec<&str>) -> Self {
        if data.len() != 9 {
            //Somehow wrong format
            DocumentListing::new()
        } else {
            //Fill in with data
            let mut doc = DocumentListing::new();
            let mut param_iter = data.iter();
            doc.id = param_iter.next().unwrap_or(&"ERR").to_string();
            doc.authors = param_iter.next().unwrap_or(&"ERR").to_string();
            doc.title = param_iter.next().unwrap_or(&"ERR").to_string();
            doc.publisher = param_iter.next().unwrap_or(&"ERR").to_string();
            doc.year_published = param_iter.next().unwrap_or(&"1337").parse().unwrap_or(0);
            doc.pages = param_iter.next().unwrap_or(&"ERR").to_string();
            doc.language = param_iter.next().unwrap_or(&"ERR").to_string();
            doc.file_size = param_iter.next().unwrap_or(&"ERR").to_string();
            doc.extension = param_iter.next().unwrap_or(&"ERR").to_string();

            doc
        }
    }

    pub fn new() -> Self {
        DocumentListing {
            id: ("".to_owned()),
            authors: ("".to_owned()),
            title: ("".to_owned()),
            publisher: ("".to_owned()),
            year_published: (0),
            pages: ("".to_owned()),
            language: ("".to_owned()),
            file_size: ("".to_owned()),
            extension: ("".to_owned()),
        }
    }
}
