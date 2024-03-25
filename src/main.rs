use std::path::{Path, PathBuf};
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
    
    /// isbn search query
    #[arg(short = 'i', long = "isbn", required = false, default_value_t = String::new())]
    isbn: String,

    /// title search query
    #[arg(short = 't', long = "title", required = false, default_value_t = String::new())]
    title: String,

    /// index of query result to download (starting at 0)
    #[arg(short = 'c', long = "choice", required = false, default_value_t = -1)]
    choice: i32,

    /// filepath or directory to put downloaded document
    #[arg(short = 'o', long = "output", required = false, default_value_t = String::new())]
    output: String,

    /// number of query results to show (a high number may result in slow load time)
    #[arg(short = 'n', long = "num-results", required = false, default_value_t = 30)]
    num_results: u32,
}

#[derive(Clone, Debug)]
enum SearchQuery{
    ISBN(String),
    TITLE(String),
}

#[derive(Debug)]
struct CLIOptions{
    query: SearchQuery,
    choice: Option<usize>,
    output: PathBuf,
    num_results: u32,
}

impl CLIOptions{
    fn new(args: Args) -> Result<CLIOptions, String>{
        // bad input checking
        if args.quick{
            return Err("Cannot create CLIOptions as user selected quick mode.".into());
        }
        if args.isbn.is_empty() && args.title.is_empty(){
            return Err("Please enter either an ISBN or title query with the -i (--isbn) or -t (--title) flags.".into());
        }
        if !args.isbn.is_empty() && !args.title.is_empty(){
            return Err("Please only specify either an ISBN with the -i (--isbn) flag or a title with the -t (--title) flag, not both".into());
        }
        if args.output.is_empty(){
            return Err("Please specify an output folder path with the -o (--output) flag.".into());
        }
        if args.num_results == 0{
            return Err("Please specify a number of search results greater than 0 with the -n (--num-results) flag.".into());
        }

        // warnings and notifications
        if args.choice == -1{
            println!("No choice selected, listing query results. If you would like to choose one of these results, run the same command with the -c (--choice) option and the index of the option you'd like.");
        }

        // file path checking
        let opt_path = handle_output_path(&args)?;        
        let buf = opt_path.ok_or("Please specify an ouput file path with -o (--output) or use quick mode with -q (--quick)")?;
        

        // return parsed ok result
        Ok(CLIOptions{
            query: if args.isbn.is_empty() {SearchQuery::TITLE(args.title)} else {SearchQuery::ISBN(args.isbn)},
            choice: if args.choice == -1 {None} else {Some(args.choice as usize)},
            output: buf,
            num_results: args.num_results
        })
    }
}

#[derive(Debug)]
struct QuickOptions {
    query: Option<SearchQuery>,
    choice: Option<usize>,
    output: Option<PathBuf>,
    num_results: u32
}

impl QuickOptions{
    fn new (args: Args) -> Result<QuickOptions, String>{
        if !args.quick{
            return Err("Cannot create QuickOptions as user did not select quick mode.".into());
        }
        if !args.isbn.is_empty() && !args.title.is_empty(){
            return Err("Please only specify either an ISBN with the -i (--isbn) flag or a title with the -t (--title) flag, not both".into());
        }
        if args.num_results == 0{
            return Err("Please specify a number of search results greater than 0 with the -n (--num-results) flag.".into());
        }

        // parsing and validating the output path

        // file path checking and error propagation
        let opt_path = handle_output_path(&args)?;

        Ok(QuickOptions{
            query: if args.isbn.is_empty() && args.title.is_empty() {
                None
            }
            else if !args.title.is_empty() {
                Some(SearchQuery::TITLE(args.title))
            }
            else {
                Some(SearchQuery::ISBN(args.isbn))
            },
            choice: if args.choice == -1 {None} else {Some(args.choice as usize)},
            output: opt_path,
            num_results: args.num_results
        })
    }
}

fn handle_output_path(args: &Args) -> Result<Option<PathBuf>, String>{
    Ok(if args.output.is_empty() {None} else {
        let path = Path::new(args.output.as_str());
        let buf_res = path.canonicalize();
        let buf;
    
        match buf_res{
            Err(err) => {
                return Err(format!("System error found trying to parse output folder path: {}", err));
            },
            Ok(val) => {
                buf = val;
            }
        }
        if !buf.is_dir(){
            return Err("Please specify a folder with the -o (--output) flag. Your file will be downloaded into that folder.".into());
        }
        Some(buf)
    })
}

#[derive(Debug)]
enum Options{
    QUICK(QuickOptions),
    CLI(CLIOptions)
}



#[tokio::main]
async fn main() -> Result<(), String>{
    //Read the input args
    let args = Args::parse();
    //dbg!(&args);


    // unwrap is fine here as we want these errors reported to the user
    let options = if args.quick{
        Options::QUICK(QuickOptions::new(args)?)
    }
    else{
        Options::CLI(CLIOptions::new(args)?)
    };
    
    //dbg!(&options);
    // unpack or request query
    let query = match &options{
        Options::CLI(o) => o.query.clone(),
        Options::QUICK(o) => match &o.query{
            Some(s) => s.clone(),
            None => {
                // choose isbn or title search
                let search_options = vec!["ISBN", "Title"];
                let result = Select::new("How would you like to search?", search_options).prompt().unwrap();
    
                match result{
                    "ISBN" => {
                        let isbn = Text::new("What ISBN would you like to find?").prompt().unwrap();
                        println!("Valid isbn, searching...");
                        SearchQuery::ISBN(isbn)
                    },
                    _ => {
                        let title = Text::new("What title would you like to find?").prompt().unwrap();
                        println!("Valid title, searching...");
                        SearchQuery::TITLE(title)
                    }
                }    
            }
        }
    };
    

    //Start a request
    let client = reqwest::Client::new();
    let host = find_hostname(&client).await.unwrap();

    let url: String = format!("{0}{1}", host, format_url(&query).unwrap());

    println!("Querying: {}", url);

    let response = client.get(url).send().await.unwrap();
    let num_results = match &options{
        Options::CLI(o) => o.num_results,
        Options::QUICK(o) => o.num_results,
    };

    let listings: Vec<DocumentListing> = if response.status().is_success() {
            let table_data = response.text().await.unwrap();
            // dbg!(&table_data);
            let table: &String = &extract_tables(table_data.as_str())[2];
            extract_table_data(table.as_str(), &host, num_results)
    } 
    else {
        return Err("libgen request failed.".to_string());
    };
    
    
    let link = match options{
        Options::CLI(o) => {
            match o.choice{
                Some(c) => listings[c].link.to_owned(),
                None => {
                    // show listings and exit early if no choice specified
                    for (i, listing) in listings.iter().enumerate() {
                        println!("{}: {}", i, listing);
                    }
                    return Ok(())
                }
            }
        },
        Options::QUICK(o) => {
            match o.choice{
                Some(c) => listings[c].link.to_owned(),
                None => Select::new("Which document would you like?", listings).prompt().unwrap().link
            }
        }
    };

    // TODO:
    // use link to make another request, then output it to chosen directory with default file name
    // or chosen file name, or make interactive file choice if using -q flag

    println!("DEBUG: Successfully chose document at link: {}", link);

    Ok(())
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

fn extract_table_data(raw_html: &str, host: &str, num_results: u32) -> Vec<DocumentListing> {
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
        for (i, row) in rows.iter().skip(1).enumerate() {
            if i >= num_results as usize {break;}
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

fn format_url(query: &SearchQuery) -> Result<String, &'static str> {
    match query{
        SearchQuery::ISBN(isbn) => {
            match isbn.as_str(){
                "" => Err("Please enter a non-empty ISBN"),
                _ => {
                    Ok(format!(
                        "/search.php?req={}&open=0&res=100&view=simple&phrase=1&column=identifier",
                        isbn
                    ))
                }
            }
        },
        SearchQuery::TITLE(title) => {
            match title.as_str(){
                "" => Err("Please enter a non-empty title"),
                _ => {
                    Ok(format!(
                        "/search.php?req={}&open=0&res=100&view=simple&phrase=1&column=title",
                        title.replace(" ", "+").as_str()
                    ))
                }
            }
            
        }
    }
}
