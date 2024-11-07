// use reqwest::Client;
// use std::path::{Path, PathBuf};
// use tokio::fs;
//
// use super::{
//     downloader::{download_region, Region},
//     voxelizer::height_data_to_voxels,
// };
//
// pub async fn load_terrain_data(
//     assets_dir: &Path,
// ) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
//     let mut height_data = Vec::new();
//
//     let mut entries = fs::read_dir(assets_dir).await?;
//     while let Some(entry) = entries.next_entry().await? {
//         let path = entry.path();
//
//         if path.extension().and_then(|s| s.to_str()) == Some("zip") {
//             let file = tokio::fs::File::open(&path).await?;
//             let mut buffer = Vec::new();
//             tokio::io::copy(&mut tokio::io::BufReader::new(file), &mut buffer).await?;
//
//             let elevations = process_hgt_data(&buffer)?;
//             height_data.push(elevations);
//         }
//     }
//
//     Ok(height_data)
// }
//
// fn process_hgt_data(data: &[u8]) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
//     let mut elevations = Vec::new();
//
//     for chunk in data.chunks(2) {
//         if chunk.len() == 2 {
//             let elevation = i16::from_be_bytes([chunk[0], chunk[1]]) as f32;
//             elevations.push(elevation);
//         }
//     }
//
//     Ok(elevations)
// }
//
// pub async fn load_or_download_terrain(
//     bounds: Region,
// ) -> Result<Vec<Vec<Vec<u32>>>, Box<dyn std::error::Error>> {
//     let assets_dir = PathBuf::from("assets/terrain");
//     fs::create_dir_all(&assets_dir).await?;
//
//     let client = Client::builder().build()?;
//     download_region(&client, &assets_dir, &bounds).await?;
//
//     let height_data = load_terrain_data(&assets_dir).await?;
//
//     let voxels = height_data_to_voxels(height_data, 32);
//
//     Ok(voxels)
// }
