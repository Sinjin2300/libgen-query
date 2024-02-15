use clap::Parser;
use doc_listing::DocumentListing;
use reqwest;
use scraper::{Html, Selector};
use serde_json;
use inquire::{Select, Text};

mod doc_listing;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// whether to use interactive search
    #[arg(short = 'q', long = "quick", required = false, default_value_t = false)]
    quick: bool,
    
    /// search by isbn
    #[arg(short = 'I', long = "ISBN", required = false, default_value_t = String::new())]
    isbn: String,

    /// search by title
    #[arg(short = 't', long, required = false, default_value_t = String::new())]
    title: String,

    /// index of option to download from search info
    #[arg(short = 'c', long = "choice", required = false, requires = "output", default_value_t = -1)]
    choice: i32,

    /// filepath or directory to put downloaded document
    #[arg(short = 'o', long = "output")]
    output: String,
}

#[tokio::main]
async fn main() {
    //Read the input args
    let mut args = Args::parse();
    dbg!(&args);

    if args.quick{
        // choose isbn or title search
        let search_options = vec!["ISBN", "Title"];
        let result = Select::new("How would you like to search?", search_options).prompt().unwrap();

        match result{
            "ISBN" => {
                let isbn = Text::new("What ISBN would you like to find?").prompt().unwrap();
                println!("Valid isbn, searching...");
                args.isbn = isbn;
            },
            _ => {
                let title = Text::new("What title would you like to find?").prompt().unwrap();
                println!("Valid title, searching...");
                args.title = title;
            }
        }
    }

    //Start a request
    let client = reqwest::Client::new();
    let host = find_hostname(&client).await.unwrap();

    let url: String = format!("{0}{1}", host, format_url(&args).unwrap());

    println!("Querying: {}", url);

    let response = client.get(url).send().await.unwrap();

    let listings: Vec<DocumentListing>;
    if response.status().is_success() {
        let table_data = response.text().await.unwrap();
        // dbg!(&table_data);
        let table: &String = &extract_tables(table_data.as_str())[2];
        listings = extract_table_data(table.as_str(), &host);
    } else {
        println!("Could not Query");
        std::process::exit(-1);
    }
    
    // if no choice, just print options
    let link: String;
    if args.choice == -1 && !args.quick{
        for (i, listing) in listings.iter().enumerate() {
            println!("{}: {}", i, listing);
        }
        return;
    }
    else if args.quick{
        link = Select::new("Which document would you like?", listings).prompt().unwrap().link;
    }
    else {
        link = listings[args.choice as usize].link.to_owned();
    }

    // TODO:
    // use link to make another request, then output it to chosen directory with default file name
    // or chosen file name, or make interactive file choice if using -q flag

    
}

async fn find_hostname(client: &reqwest::Client) -> Result<String, &'static str> {
    let response = client
        .get("https://whereislibgen.vercel.app/api")
        .send()
        .await
        .unwrap();

    // Check if the request was successful (status code 2xx)
    if response.status().is_success() {
        // Read the response body as a string
        let body = response.text().await.unwrap();
        let hosts: Vec<String> = serde_json::from_str(&body).unwrap();
        dbg!(&hosts);
        let mut host: Option<String> = None;
        for url in hosts {
            match test_connection(url, client).await {
                Err(..) => continue,
                Ok(url) => {
                    host = Some(url);
                    break;
                }
            }
        }
        match host {
            None => return Err("Cannot Find Host"),
            Some(url) => return Ok(url),
        }
    } else {
        Err("No Response")
    }
}

fn extract_table_data(raw_html: &str, host: &str) -> Vec<DocumentListing> {
    let document = Html::parse_document(raw_html);
    let mut output: Vec<DocumentListing> = Vec::new();
    // Select the table based on its attributes
    let table_selector = Selector::parse(
        "table[width=\"100%\"][cellspacing=\"1\"][cellpadding=\"1\"][rules=\"rows\"][class=\"c\"]",
    )
    .unwrap();

    // Check if the table exists
    if let Some(table) = document.select(&table_selector).next() {
        // Collect rows into a Vec before iterating
        let rows: Vec<_> = table.select(&Selector::parse("tr").unwrap()).collect();

        // Iterate over the rows starting from the second one
        for row in rows.iter().skip(1) {
            // Process each row as needed
            let mut items: Vec<String> = row
                .text()
                .map(|x| x.replace("\n\t\t\t\t", "|"))
                .collect::<String>()
                .split_terminator('|')
                .take(9)
                .map(String::from)
                .collect();

            items.push(format!(
                "{}/{}",
                host,
                find_link_by_id(raw_html, &items[0]).unwrap()
            ));
            output.push(DocumentListing::from(&items));
        }
    } else {
        println!("Table not found");
    }
    output
}

fn find_link_by_id(html: &str, target_id: &str) -> Option<String> {
    let document = Html::parse_document(html);

    // Construct a CSS selector to select the link with the specified id
    let selector_str = format!("a[id=\"{}\"]", target_id);
    let link_selector = Selector::parse(&selector_str).unwrap();

    // Find the link using the selector
    if let Some(link) = document.select(&link_selector).next() {
        // Get the value of the "href" attribute
        let href_attribute = link.value().attr("href");
        href_attribute.map(String::from)
    } else {
        None
    }
}

fn extract_tables(raw_html: &str) -> Vec<String> {
    let document = Html::parse_document(raw_html);

    // Select all HTML tables using a CSS selector
    let table_selector = Selector::parse("table").unwrap();

    // Extract the inner HTML of each table
    document
        .select(&table_selector)
        .map(|table| table.html())
        .collect()
}

async fn test_connection(url: String, client: &reqwest::Client) -> Result<String, &'static str> {
    let response = client.get(&url).send().await;
    match response {
        Ok(response) => {
            if response.status().is_success() {
                Ok(url)
            } else {
                Err("No Response")
            }
        }
        Err(..) => Err("Cannot Reach"),
    }
}

fn format_url(args: &Args) -> Result<String, &str> {
    match args.isbn.as_str() {
        "" => match args.title.as_str() {
            "" => Err("No Search Parameters"),
            _ => {
                //Search with a title
                Ok(format!(
                    "/search.php?req={}&open=0&res=100&view=simple&phrase=1&column=title",
                    args.title.replace(" ", "+").as_str()
                ))
            }
        },
        _ => {
            //Search with an ISBN
            Ok(format!(
                "/search.php?req={}&open=0&res=100&view=simple&phrase=1&column=identifier",
                args.isbn.as_str()
            ))
        }
    }
}
