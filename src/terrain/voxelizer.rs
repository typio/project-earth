use std::collections::HashMap;

// negative for west and south
#[derive(Debug)]
struct GeoCoord {
    lat: f32,
    lon: f32,
}

#[derive(Debug, Clone)]
struct VoxelMaterial {
    id: u32,
    albedo: [f32; 3],
}

#[derive(Debug, Clone)]
struct Voxel {
    height: u32,
    material: VoxelMaterial,
}

#[derive(Debug)]
struct VoxelChunk {
    coord: GeoCoord,
    voxels: Vec<Vec<Vec<u32>>>,
}

#[derive(Debug)]
struct VoxelWorld {
    materials: Vec<VoxelMaterial>,
    voxel_length: f32,
    chunk_size: Vec<u32>,
    chunks: HashMap<GeoCoord, VoxelChunk>,
}

// const AIR: VoxelMaterial = VoxelMaterial {
//     id: 0,
//     albedo: [0.0, 0.0, 0.0],
// };
//
// const WATER: VoxelMaterial = VoxelMaterial {
//     id: 1,
//     albedo: [0.02, 0.03, 0.1],
// };
//
// const GRASS: VoxelMaterial = VoxelMaterial {
//     id: 2,
//     albedo: [0.1, 0.3, 0.05],
// };
//
// const DIRT: VoxelMaterial = VoxelMaterial {
//     id: 3,
//     albedo: [0.25, 0.15, 0.08],
// };
//
// const STONE: VoxelMaterial = VoxelMaterial {
//     id: 4,
//     albedo: [0.3, 0.3, 0.32],
// };
//
// const LENGTH: usize = 32;
// const WIDTH: usize = 32;
// const HEIGHT: usize = 32;
//
// pub fn height_data_to_voxels(
//     height_data: Vec<Vec<f32>>,
//     max_height: usize,
// ) -> Vec<Vec<Vec<VoxelMaterial>>> {
//     let size = height_data.len();
//     let mut voxels = vec![vec![vec![AIR; size]; max_height]; size];
//
//     // Find min/max heights for normalization
//     let mut min_height = f32::INFINITY;
//     let mut max_height_found = f32::NEG_INFINITY;
//
//     for row in &height_data {
//         for &height in row {
//             min_height = min_height.min(height);
//             max_height_found = max_height_found.max(height);
//         }
//     }
//
//     let height_range = max_height_found - min_height;
//
//     // Fill voxels
//     for x in 0..size {
//         for z in 0..size {
//             let normalized_height = ((height_data[z][x] - min_height) / height_range
//                 * (max_height as f32 - 1.0)) as usize;
//
//             // Add terrain layers
//             for y in 0..=normalized_height {
//                 voxels[x][y][z] = if y == normalized_height {
//                     GRASS
//                 } else if y > normalized_height - 3 {
//                     DIRT
//                 } else {
//                     STONE
//                 };
//             }
//
//             // Add water at a certain level if desired
//             let water_level = max_height / 3;
//             if normalized_height < water_level {
//                 for y in normalized_height + 1..=water_level {
//                     voxels[x][y][z] = WATER;
//                 }
//             }
//         }
//     }
//
//     voxels
// }

const LENGTH: usize = 32;
const HEIGHT: usize = 32;
const WIDTH: usize = 32;

const AIR: u32 = 0;
const WATER: u32 = 1;
const GRASS: u32 = 2;
const DIRT: u32 = 3;
const STONE: u32 = 4;

pub fn generate_volume() -> [[[u32; WIDTH]; HEIGHT]; LENGTH] {
    // Changed array dimension order
    let mut volume = [[[AIR; WIDTH]; HEIGHT]; LENGTH]; // Changed array dimension order
    let water_level = (HEIGHT * 2) / 3;
    let river_width = LENGTH / 8;
    let river_center = LENGTH / 2;

    for x in 0..LENGTH {
        for y in 0..HEIGHT {
            // Swapped y and z loop order
            for z in 0..WIDTH {
                // Calculate terrain height for this x,z coordinate
                let column_height = {
                    let base_height = HEIGHT - 12;
                    let wave1 = ((x as f32 * 0.2).sin() * 4.0) as usize;
                    let wave2 = ((z as f32 * 0.3).sin() * 3.0) as usize;
                    base_height + wave1 + wave2
                };

                let is_river = (x as i32 - river_center as i32).abs() < river_width as i32;

                if is_river && y <= water_level {
                    volume[x][y][z] = WATER; // Changed array indexing order
                } else if !is_river && y <= column_height {
                    let material = if y == column_height {
                        GRASS
                    } else if y > column_height - 3 {
                        DIRT
                    } else {
                        STONE
                    };
                    volume[x][y][z] = WATER; // Changed array indexing order
                }
            }
        }
    }

    volume
}
