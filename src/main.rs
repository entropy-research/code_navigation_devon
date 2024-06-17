use anyhow::Result;


#[tokio::main]
async fn main() -> Result<()> {
    // let root_path = Path::new("/Users/arnav/Desktop/devon/Devon");
    // // println!("{}", root_path.display());
    // let index_path = Path::new("/Users/arnav/Desktop/devon/Devon/index");
    
    // let buffer_size_per_thread = 15_000_000;
    // let num_threads = 4;

    // let indexes = Indexes::new(&index_path, buffer_size_per_thread, num_threads).await?;
    // indexes.index(root_path).await?;

    // // // // // Create a searcher and perform a search
    // let searcher = Searcher::new(&index_path)?;
    // let result = searcher.token_info("/Users/arnav/Desktop/devon/Devon/devon_agent/agents/default/agent.py", 33, 6, 11);
    // match result {
    //     Ok(token_info) => println!("{}", pyo3_example::search::Searcher::format_token_info(token_info)),
    //     Err(e) => println!("Error retrieving token info: {}", e),
    // }

    // // let result = searcher.text_search("indexes")?;
    // // println!("{}", retreival::search::Searcher::format_search_results(result));

    // // println!("-");
    // // // Print out the results

    Ok(())
}
