use bevy::prelude::*;

const PRIME_X: u32 = 1;
const PRIME_Y: u32 = 2654435761;

/// Spatial Partitioning Grid for O(N) neighbor lookup
#[derive(Debug, Clone)]
pub struct SpatialHashGrid {
    table_size: usize,
    cells: Vec<Vec<usize>>,
    cell_size: f32,
}

impl SpatialHashGrid {
    pub fn new(interaction_radius: f32) -> Self {
        // Cell size should be at least the max interaction radius
        let cell_size = interaction_radius.max(1.0);
        let table_size = 100000; // Large enough hash table to minimize collisions
        
        Self {
            table_size,
            cells: vec![Vec::new(); table_size],
            cell_size,
        }
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.clear();
        }
    }

    fn hash_coords(&self, x: u32, y: u32) -> usize {
        let hash = (x.wrapping_mul(PRIME_X)) ^ (y.wrapping_mul(PRIME_Y));
        (hash as usize) % self.table_size
    }

    fn pos_to_coords(&self, pos: Vec2) -> (u32, u32) {
        // Use i32 to gracefully handle negative coordinates for infinite space
        let x0 = (pos.x / self.cell_size).floor() as i32 as u32;
        let y0 = (pos.y / self.cell_size).floor() as i32 as u32;
        (x0, y0)
    }

    pub fn insert(&mut self, index: usize, pos: Vec2) {
        let (x, y) = self.pos_to_coords(pos);
        let idx = self.hash_coords(x, y);
        self.cells[idx].push(index);
    }

    pub fn get_neighbors(&self, pos: Vec2) -> impl Iterator<Item = &usize> {
        let (x, y) = self.pos_to_coords(pos);
        let mut neighbors = Vec::new();
        
        // Search 3x3 grid around the cell
        for dy in 0..=2 {
            for dx in 0..=2 {
                let nx = x.wrapping_add(dx).wrapping_sub(1);
                let ny = y.wrapping_add(dy).wrapping_sub(1);
                let idx = self.hash_coords(nx, ny);
                neighbors.push(&self.cells[idx]);
            }
        }
        
        neighbors.into_iter().flat_map(|cell| cell.iter())
    }
}

/// Continuous Density Grid using Bilinear Interpolation from hg-network
#[derive(Debug, Clone)]
pub struct DensityGrid {
    table_size: usize,
    features: Vec<f32>,
    cell_size: f32,
}

impl DensityGrid {
    pub fn new(cell_size: f32) -> Self {
        let table_size = 100000;
        Self {
            table_size,
            features: vec![0.0; table_size],
            cell_size,
        }
    }

    pub fn clear(&mut self) {
        self.features.fill(0.0);
    }

    #[inline]
    fn hash_coords(&self, x: u32, y: u32) -> usize {
        let hash = (x.wrapping_mul(PRIME_X)) ^ (y.wrapping_mul(PRIME_Y));
        (hash as usize) % self.table_size
    }

    pub fn add_density(&mut self, pos: Vec2, amount: f32) {
        let scaled_x = pos.x / self.cell_size;
        let scaled_y = pos.y / self.cell_size;

        let x0_i = scaled_x.floor() as i32;
        let y0_i = scaled_y.floor() as i32;
        let x0 = x0_i as u32;
        let y0 = y0_i as u32;
        let x1 = (x0_i + 1) as u32;
        let y1 = (y0_i + 1) as u32;

        let wx = scaled_x - x0_i as f32;
        let wy = scaled_y - y0_i as f32;

        let idx_00 = self.hash_coords(x0, y0);
        let idx_10 = self.hash_coords(x1, y0);
        let idx_01 = self.hash_coords(x0, y1);
        let idx_11 = self.hash_coords(x1, y1);

        // Splat density with bilinear weights
        self.features[idx_00] += amount * (1.0 - wx) * (1.0 - wy);
        self.features[idx_10] += amount * wx * (1.0 - wy);
        self.features[idx_01] += amount * (1.0 - wx) * wy;
        self.features[idx_11] += amount * wx * wy;
    }

    pub fn evaluate(&self, pos: Vec2) -> f32 {
        let scaled_x = pos.x / self.cell_size;
        let scaled_y = pos.y / self.cell_size;

        let x0_i = scaled_x.floor() as i32;
        let y0_i = scaled_y.floor() as i32;
        let x0 = x0_i as u32;
        let y0 = y0_i as u32;
        let x1 = (x0_i + 1) as u32;
        let y1 = (y0_i + 1) as u32;

        let wx = scaled_x - x0_i as f32;
        let wy = scaled_y - y0_i as f32;

        let idx_00 = self.hash_coords(x0, y0);
        let idx_10 = self.hash_coords(x1, y0);
        let idx_01 = self.hash_coords(x0, y1);
        let idx_11 = self.hash_coords(x1, y1);

        let v00 = self.features[idx_00];
        let v10 = self.features[idx_10];
        let v01 = self.features[idx_01];
        let v11 = self.features[idx_11];

        let bottom = v00 * (1.0 - wx) + v10 * wx;
        let top = v01 * (1.0 - wx) + v11 * wx;
        bottom * (1.0 - wy) + top * wy
    }
}
