use futures::future::join_all;
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::env;
use std::io::prelude::*;
use anyhow::Result;
use tokio;

mod googler;
mod simparse;
mod gptcall;
mod gptextract;
mod my_secret;
mod types;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: cargo run <query> <comma-delimited-entities>");
        return Ok(());
    }
    
    let query = &args[1];
    let entities: &Vec<&str> = &args[2].split(',').collect();

    println!("{}", query);
    // Step 1: Google Shallow
    let search_results = googler::search_query(&query).await?;
    let parsed_results = googler::parse_google_results(&search_results);

    println!("{:#?}", parsed_results);
    let json_string = serde_json::to_string(&parsed_results)?;
    
    let json_value: Value = serde_json::from_str(&json_string)?;
    let links = json_value["links"].as_array().unwrap();
    let content = json_value["content"].as_str();
    
    // // Step 2: Consistency Check
    // let consistency_check_prompt = format!("Check consistency: {:#?}", content);
    // let system_prompt = "You are a helpful assistant.";
    // let consistency_response = gptcall::call_openai_chat(system_prompt, &consistency_check_prompt, my_secret::OPENAI_KEY).await?;
    
    // // Print consistency check response
    // println!("Consistency Check Response: {}", consistency_response);
    
    // Step 3: Google Deep
    // let mut all_extracted_data: Vec<_> = vec![];
    let mut all_extracted_data: Vec<serde_json::Value> = vec![];
    
    let tags = vec!["h1", "h2", "h3", "h4", "p", "article", "td", "ul", "li", "lo"];
    let proxy_url: Option<&str> = None;
    // let proxy_url: &str = "socks5h://127.0.0.1:9050";
    
    let tasks: Vec<_> = links.iter().map(|link| {
        let url = link.as_str().expect("Expected a valid URL");
        let tags = tags.clone();
        let proxy_url = proxy_url.clone();
    
        async move {
            match simparse::fetch_and_extract(url, tags, proxy_url).await {
                Ok(results) => {
                    let json_results: Vec<_> = results.iter()
                        .map(|result| serde_json::json!({ "tag": &result.tag, "value": &result.value }))
                        .collect();
    
                    println!("{}", serde_json::to_string_pretty(&json_results).unwrap());
    
                    let extraction_input = serde_json::to_string(&json_results).unwrap();
    
                    match gptextract::information_extraction(&extraction_input, Some(&entities), proxy_url).await {
                        Ok(extraction_response) => {
                            println!("Extracted Information: {}", extraction_response);
    
                            // Log extraction_response for debugging
                            println!("Raw extraction_response: {}", extraction_response);
    
                            // Parse the extraction_response as JSON
                            match serde_json::from_str::<serde_json::Value>(&extraction_response) {
                                Ok(extraction_json) => {
                                    // Extract nodes and edges
                                    let nodes = extraction_json["nodes"].as_array().unwrap_or(&vec![]).clone();
                                    let edges = extraction_json["edges"].as_array().unwrap_or(&vec![]).clone();
                                    Ok((nodes, edges))
                                },
                                Err(err) => {
                                    eprintln!("JSON parsing error: {}", err);
                                    Err(anyhow::anyhow!(err.to_string()))
                                }
                            }
                        },
                        Err(err) => {
                            eprintln!("Extraction error: {}", err);
                            Err(anyhow::anyhow!(err.to_string()))
                        }
                    }
                },
                Err(err) => {
                    eprintln!("Fetch error: {}", err);
                    Err(anyhow::anyhow!(err.to_string()))
                }
            }
        }
    }).collect();
    
    let results = join_all(tasks).await;
    
    // Aggregate results into a single object with 'nodes' and 'edges'
    let mut aggregated_results = serde_json::json!({
        "nodes": [],
        "edges": []
    });
    
    for result in results {
        match result {
            Ok((nodes, edges)) => {
                aggregated_results["nodes"].as_array_mut().unwrap().extend(nodes);
                aggregated_results["edges"].as_array_mut().unwrap().extend(edges);
            },
            Err(err) => {
                eprintln!("Task error: {}", err);
            }
        }
    }

    // let results = join_all(tasks).await;

    // for result in results {
    //     match result {
    //         Ok(data) => all_extracted_data.push(data),
    //         Err(_) => eprintln!("An error occurred during processing."),
    //     }
    // }

    
    // Step 4: Saving
    // Define the file path
    let file_path = "data/results.json";
    
    // Serialize all extracted data to JSON
    let results_json = serde_json::to_string_pretty(&aggregated_results)?;
    
    // Write the JSON string to the file
    let mut file = File::create(file_path)?;
    file.write_all(results_json.as_bytes())?;
    
    println!("Results saved to '{}'", file_path);
    
    Ok(())
}
