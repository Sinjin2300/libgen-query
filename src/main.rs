use clap::Parser;
use reqwest;
use scraper::{Html, Selector};
use serde_json;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'I', long = "ISBN", requires = "directory", required = false, default_value_t = String::new())]
    isbn: String,

    #[arg(required = false, short = 't', long, requires = "directory", default_value_t = String::new())]
    title: String,

    #[arg(short = 'o', long = "out")]
    directory: String,
}

#[tokio::main]
async fn main() {
    //Read the input args
    let args = Args::parse();
    dbg!(&args);

    //Start a request
    let client = reqwest::Client::new();

    let url: String = format!(
        "{0}{1}",
        find_hostname(&client).await.unwrap(),
        format_url(&args).unwrap()
    );

    let response = client.get(url).send().await.unwrap();

    if response.status().is_success() {
        let tables: Vec<String> = extract_tables(response.text().await.unwrap().as_str());
        dbg!(&tables[2]);
    } else {
        println!("Could not Query")
    }
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
                    args.isbn.replace(" ", "+").as_str()
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
