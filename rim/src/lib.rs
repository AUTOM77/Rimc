pub mod llm;
pub mod client;
pub mod modality;

use futures::StreamExt;

async fn caption(
    m: &modality::Media,
    clt: &client::RimClient,
    idx: usize
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let _data = m.data().await?;
    let mime = m.get_mime();
    let m_url = clt.upload_asset(_data, &mime).await?;
    let _delay = (idx % 100) * 200;
    let mut retries = 0;
    let _cap = loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(_delay as u64)).await;
        match clt.generate_caption(m_url.clone(), mime.clone()).await {
            Ok(res) => break res,
            Err(e) => {
                println!("Retry {:#?} with {:?} times", idx, retries);
                retries += 1;
                tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
                if retries > 10 {
                    print!("Failed Path: {:#?}", m.log_file());
                    return Err(e);
                }
            }
        };
    };
    let _ = m.save(_cap).await?;
    clt.log_api();
    print!("Success Path: {:#?}", m.log_file());
    Ok(idx)
}

async fn processing(
    media: Vec<modality::Media>,
    client: client::RimClient,
    limit: usize
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut tasks = futures::stream::FuturesUnordered::new();
    let mut num = 0;

    for chunk in media.chunks(limit) {
        for m in chunk {
            let clt = &client;
            tasks.push(caption(m, clt, num));
            num += 1;
        }

        while let Some(handle) = tasks.next().await {
            let _ = match handle {
                Ok(i) => eprintln!("Success: {:?}", i),
                Err(e) => eprintln!("Task failed: {:?}", e),
            };
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        tasks.clear();
    }
    Ok(())
}

pub fn _rt2(pth: &str, prj: String, key:String, prompt: String, limit: Option<usize>) -> Result<(), Box<dyn std::error::Error>> {
    let client = client::RimClient::build(prompt, prj).with_auth(key);

    let media: Vec<modality::Media> = std::fs::read_dir(pth)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path().display().to_string())
        .map(|f| modality::Media::from(&f).unwrap())
        .filter(|i| !i.existed())
        .collect();
    
    println!("Processing Media {:#?}", media.len());
    std::thread::sleep(std::time::Duration::from_secs(1));
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    match limit {
        Some(n) => rt.block_on(processing(media, client, n)),
        None => rt.block_on(processing(media, client, 1000))
    };
    Ok(())
}
