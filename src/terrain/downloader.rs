use futures::StreamExt;
use reqwest::Client;
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;

// struct EarthdataCredentials {
//     username: String,
//     password: String,
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let credentials = EarthdataCredentials {
    //     username: "your_username".to_string(),
    //     password: "your_password".to_string(),
    // };

    // Create assets directory if it doesn't exist
    let assets_dir = PathBuf::from("assets/terrain");
    fs::create_dir_all(&assets_dir).await?;
    println!("Assets directory: {}", assets_dir.display());

    let bounds = Region {
        north: 49,
        south: 45,
        east: 5,
        west: 0,
    };

    // Download tiles
    let client = Client::builder().build()?;
    download_region(&client, &assets_dir, &bounds).await?;

    Ok(())
}

pub struct Region {
    north: i32,
    south: i32,
    east: i32,
    west: i32,
}

pub async fn download_region(
    client: &Client,
    assets_dir: &Path,
    bounds: &Region,
) -> Result<(), Box<dyn std::error::Error>> {
    let tiles = generate_tile_list(bounds);

    // Create a stream of concurrent downloads
    let mut downloads = futures::stream::iter(tiles.into_iter().map(|tile| {
        let client = client.clone();
        let dir = assets_dir.to_owned();

        async move {
            match download_tile_if_needed(&client, &dir, &tile).await {
                Ok(_) => println!("Tile {} ready", tile),
                Err(e) => eprintln!("Failed to process {}: {}", tile, e),
            }
        }
    }))
    .buffer_unordered(4); // Download 4 tiles concurrently

    while let Some(_) = downloads.next().await {}

    Ok(())
}

fn generate_tile_list(bounds: &Region) -> Vec<String> {
    let mut tiles = Vec::new();

    for lat in bounds.south..=bounds.north {
        for lon in bounds.west..=bounds.east {
            let ns = if lat >= 0 { "n" } else { "s" };
            let ew = if lon >= 0 { "e" } else { "w" };

            let tile_name = format!("{}{:02}{}{:03}.zip", ns, lat.abs(), ew, lon.abs());

            tiles.push(tile_name);
        }
    }

    tiles
}

async fn download_tile_if_needed(
    client: &Client,
    assets_dir: &Path,
    tile_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = assets_dir.join(tile_name);

    // Check if file already exists
    if file_path.exists() {
        println!("Tile {} already exists, skipping download", tile_name);
        return Ok(());
    }

    println!("Downloading tile {}", tile_name);

    let url = format!(
        "https://e4ftl01.cr.usgs.gov/MEASURES/NASADEM_SHHP.001/2000.02.11/NASADEM_SHHP_{}",
        tile_name
    );

    // Create authenticated request
    let response = client
        .get(&url)
        // .basic_auth(&credentials.username, Some(&credentials.password))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to download: {}", response.status()).into());
    }

    // Create file in assets directory
    let mut file = File::create(&file_path).await?;

    // Download and write the file
    let mut bytes = response.bytes().await?;
    file.write_all_buf(&mut bytes).await?;

    println!("Successfully downloaded {}", tile_name);
    Ok(())
}
