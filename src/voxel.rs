use crate::{graphics, Color, Extent3d, Offset3d, Vector2, Vector3};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Voxel {
    #[default]
    Void,
    Tile(u64),
}

pub struct WorldIterator<'a> {
    index: usize,
    world: &'a World,
}

impl<'a> WorldIterator<'a> {
    fn new(world: &'a World) -> Self {
        Self { index: 0, world }
    }
}

impl<'a> Iterator for WorldIterator<'a> {
    type Item = (Offset3d<i32>, &'a Voxel);

    fn next(&mut self) -> Option<Self::Item> {
        let position = self.world.get_voxel_position(self.index)?;
        self.index += 1;
        let voxel = self.world.get_voxel(position)?;
        Some((position, voxel))
    }
}

use graphics::mesh::Vertex;
const CUBE_VERTICES: [Vertex; 36] = [
    // left
    Vertex {
        position: Vector3::new(-0.5, -0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 1.0),
    },
    Vertex {
        position: Vector3::new(-0.5, 0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 0.0),
    },
    Vertex {
        position: Vector3::new(-0.5, -0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 1.0),
    },
    Vertex {
        position: Vector3::new(-0.5, 0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 0.0),
    },
    Vertex {
        position: Vector3::new(-0.5, 0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 0.0),
    },
    Vertex {
        position: Vector3::new(-0.5, -0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 1.0),
    },
    // right
    Vertex {
        position: Vector3::new(0.5, -0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 1.0),
    },
    Vertex {
        position: Vector3::new(0.5, 0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 0.0),
    },
    Vertex {
        position: Vector3::new(0.5, -0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 1.0),
    },
    Vertex {
        position: Vector3::new(0.5, 0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 0.0),
    },
    Vertex {
        position: Vector3::new(0.5, 0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 0.0),
    },
    Vertex {
        position: Vector3::new(0.5, -0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 1.0),
    },
    // bottom
    Vertex {
        position: Vector3::new(-0.5, -0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 1.0),
    },
    Vertex {
        position: Vector3::new(-0.5, -0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 0.0),
    },
    Vertex {
        position: Vector3::new(0.5, -0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 1.0),
    },
    Vertex {
        position: Vector3::new(-0.5, -0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 0.0),
    },
    Vertex {
        position: Vector3::new(0.5, -0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 0.0),
    },
    Vertex {
        position: Vector3::new(0.5, -0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 1.0),
    },
    // top
    Vertex {
        position: Vector3::new(-0.5, 0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 1.0),
    },
    Vertex {
        position: Vector3::new(-0.5, 0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 0.0),
    },
    Vertex {
        position: Vector3::new(0.5, 0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 1.0),
    },
    Vertex {
        position: Vector3::new(-0.5, 0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 0.0),
    },
    Vertex {
        position: Vector3::new(0.5, 0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 0.0),
    },
    Vertex {
        position: Vector3::new(0.5, 0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 1.0),
    },
    // far
    Vertex {
        position: Vector3::new(0.5, -0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 1.0),
    },
    Vertex {
        position: Vector3::new(0.5, 0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 0.0),
    },
    Vertex {
        position: Vector3::new(-0.5, -0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 1.0),
    },
    Vertex {
        position: Vector3::new(0.5, 0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 0.0),
    },
    Vertex {
        position: Vector3::new(-0.5, 0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 0.0),
    },
    Vertex {
        position: Vector3::new(-0.5, -0.5, -0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 1.0),
    },
    // near
    Vertex {
        position: Vector3::new(0.5, -0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 1.0),
    },
    Vertex {
        position: Vector3::new(0.5, 0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 0.0),
    },
    Vertex {
        position: Vector3::new(-0.5, -0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 1.0),
    },
    Vertex {
        position: Vector3::new(0.5, 0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(1.0, 0.0),
    },
    Vertex {
        position: Vector3::new(-0.5, 0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 0.0),
    },
    Vertex {
        position: Vector3::new(-0.5, -0.5, 0.5),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        uv: Vector2::new(0.0, 1.0),
    },
];

pub struct World {
    size: Extent3d<u32>,
    origin_offset: Offset3d<i32>,
    voxels: Vec<Voxel>,
    mesh: graphics::Mesh,
}

impl World {
    pub(crate) fn new(graphics: &graphics::Context, size: Extent3d<u32>) -> Self {
        let half_size = Extent3d::new(
            size.width as i64 / 2,
            size.height as i64 / 2,
            size.depth as i64 / 2,
        );
        let origin_offset = Offset3d::new(
            -half_size.width as i32,
            -half_size.height as i32,
            -half_size.depth as i32,
        );
        let voxels = vec![Voxel::default(); (size.width * size.height * size.depth) as usize];
        let mesh = graphics.create_mesh(&[]);
        Self {
            size,
            origin_offset,
            voxels,
            mesh,
        }
    }

    pub fn size(&self) -> Extent3d<u32> {
        self.size
    }

    pub fn mesh(&self) -> &graphics::Mesh {
        &self.mesh
    }

    pub fn get_voxel(&self, position: Offset3d<i32>) -> Option<&Voxel> {
        self.voxels.get(self.get_voxel_index(position)?)
    }

    pub fn set_voxel(&mut self, position: Offset3d<i32>, voxel: Voxel) -> Option<Voxel> {
        let Some(index) = self.get_voxel_index(position) else { return None; };
        let Some(old_voxel) = self.voxels.get_mut(index) else { return None; };
        *old_voxel = voxel;
        Some(*old_voxel)
    }

    pub fn iter(&self) -> WorldIterator<'_> {
        WorldIterator::new(self)
    }

    pub fn generate_mesh(&mut self) {
        let mut staging = Vec::with_capacity(CUBE_VERTICES.len());
        for (position, voxel) in self.iter() {
            if let Voxel::Void = voxel {
                continue;
            }
            let position = Vector3::new(position.x as f32, position.y as f32, position.z as f32);
            staging.extend(CUBE_VERTICES.iter().map(|vertex| Vertex {
                position: vertex.position + position,
                ..*vertex
            }));
        }
        self.mesh.vertices.extend_from_slice(&staging);
    }

    #[inline]
    fn get_voxel_index(&self, position: Offset3d<i32>) -> Option<usize> {
        let pos = Offset3d::new(
            (position.x - self.origin_offset.x) as u32,
            (position.y - self.origin_offset.y) as u32,
            (position.z - self.origin_offset.z) as u32,
        );
        if pos.x >= self.size.width || pos.y >= self.size.height || pos.z >= self.size.depth {
            return None;
        }
        unsafe { Some(self.get_voxel_index_unchecked(position)) }
    }

    #[inline]
    unsafe fn get_voxel_index_unchecked(&self, position: Offset3d<i32>) -> usize {
        let (width, height) = {
            let size = self.size();
            (size.width as usize, size.height as usize)
        };
        let (x, y, z) = {
            (
                (position.x - self.origin_offset.x) as usize,
                (position.y - self.origin_offset.y) as usize,
                (position.z - self.origin_offset.z) as usize,
            )
        };
        z * width * height + y * width + x
    }

    #[inline]
    unsafe fn get_voxel_position_unchecked(&self, index: usize) -> Offset3d<i32> {
        let index = index as i64;
        let (width, height, depth) = {
            let size = self.size();
            (size.width as i64, size.height as i64, size.depth as i64)
        };
        let (origin_x, origin_y, origin_z) = (width / 2, height / 2, depth / 2);
        let x = index % width;
        let y = (index / width) % height;
        let z = index / (width * height);
        Offset3d::new(
            (x - origin_x) as i32,
            (y - origin_y) as i32,
            (z - origin_z) as i32,
        )
    }

    #[inline]
    fn get_voxel_position(&self, index: usize) -> Option<Offset3d<i32>> {
        if index >= self.voxels.len() {
            None
        } else {
            unsafe { Some(self.get_voxel_position_unchecked(index)) }
        }
    }
}
