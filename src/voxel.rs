use crate::{graphics, impl_from_error, Color, Extent2d, Extent3d, Offset3d, Vector2, Vector3};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Voxel {
    #[default]
    Void,
    Tile(u32),
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

const CUBE_VERTEX_COUNT: usize = 36;
const CUBE_VERTEX_POSITIONS: [Vector3<f32>; CUBE_VERTEX_COUNT] = [
    // left
    Vector3::new(-0.5, -0.5, -0.5),
    Vector3::new(-0.5, 0.5, -0.5),
    Vector3::new(-0.5, -0.5, 0.5),
    Vector3::new(-0.5, 0.5, -0.5),
    Vector3::new(-0.5, 0.5, 0.5),
    Vector3::new(-0.5, -0.5, 0.5),
    // right
    Vector3::new(0.5, -0.5, -0.5),
    Vector3::new(0.5, 0.5, -0.5),
    Vector3::new(0.5, -0.5, 0.5),
    Vector3::new(0.5, 0.5, -0.5),
    Vector3::new(0.5, 0.5, 0.5),
    Vector3::new(0.5, -0.5, 0.5),
    // bottom
    Vector3::new(-0.5, -0.5, -0.5),
    Vector3::new(-0.5, -0.5, 0.5),
    Vector3::new(0.5, -0.5, -0.5),
    Vector3::new(-0.5, -0.5, 0.5),
    Vector3::new(0.5, -0.5, 0.5),
    Vector3::new(0.5, -0.5, -0.5),
    // top
    Vector3::new(0.5, 0.5, 0.5),
    Vector3::new(0.5, 0.5, -0.5),
    Vector3::new(-0.5, 0.5, 0.5),
    Vector3::new(0.5, 0.5, -0.5),
    Vector3::new(-0.5, 0.5, -0.5),
    Vector3::new(-0.5, 0.5, 0.5),
    // far
    Vector3::new(0.5, -0.5, -0.5),
    Vector3::new(0.5, 0.5, -0.5),
    Vector3::new(-0.5, -0.5, -0.5),
    Vector3::new(0.5, 0.5, -0.5),
    Vector3::new(-0.5, 0.5, -0.5),
    Vector3::new(-0.5, -0.5, -0.5),
    // near
    Vector3::new(0.5, -0.5, 0.5),
    Vector3::new(0.5, 0.5, 0.5),
    Vector3::new(-0.5, -0.5, 0.5),
    Vector3::new(0.5, 0.5, 0.5),
    Vector3::new(-0.5, 0.5, 0.5),
    Vector3::new(-0.5, -0.5, 0.5),
];
const CUBE_VERTEX_UVS: [Vector2<f32>; CUBE_VERTEX_COUNT] = [
    // -X
    Vector2::new(0.0, 0.667),
    Vector2::new(0.0, 0.333),
    Vector2::new(1.0, 0.667),
    Vector2::new(0.0, 0.333),
    Vector2::new(1.0, 0.333),
    Vector2::new(1.0, 0.667),
    // +X
    Vector2::new(1.0, 0.667),
    Vector2::new(1.0, 0.333),
    Vector2::new(0.0, 0.667),
    Vector2::new(1.0, 0.333),
    Vector2::new(0.0, 0.333),
    Vector2::new(0.0, 0.667),
    // -Y
    Vector2::new(0.0, 1.0),
    Vector2::new(0.0, 0.667),
    Vector2::new(1.0, 1.0),
    Vector2::new(0.0, 0.667),
    Vector2::new(1.0, 0.667),
    Vector2::new(1.0, 1.0),
    // +Y
    Vector2::new(1.0, 0.333),
    Vector2::new(1.0, 0.0),
    Vector2::new(0.0, 0.333),
    Vector2::new(1.0, 0.0),
    Vector2::new(0.0, 0.0),
    Vector2::new(0.0, 0.333),
    // -Z
    Vector2::new(0.0, 0.667),
    Vector2::new(0.0, 0.333),
    Vector2::new(1.0, 0.667),
    Vector2::new(0.0, 0.333),
    Vector2::new(1.0, 0.333),
    Vector2::new(1.0, 0.667),
    // +Z
    Vector2::new(1.0, 0.667),
    Vector2::new(1.0, 0.333),
    Vector2::new(0.0, 0.667),
    Vector2::new(1.0, 0.333),
    Vector2::new(0.0, 0.333),
    Vector2::new(0.0, 0.667),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextureLayout {
    #[default]
    Single,
}

#[derive(Debug, PartialEq)]
pub enum WorldError {
    PositionInvalid(Offset3d<i32>),
    TileIndexInvalid(u32),
    DataInvalid,
    Graphics(graphics::Error),
    Texture(graphics::texture::Error),
}

impl_from_error!(graphics::Error, WorldError, Graphics);
impl_from_error!(graphics::texture::Error, WorldError, Texture);

pub struct World {
    size: Extent3d<u32>,
    origin_offset: Offset3d<i32>,
    voxels: Vec<Voxel>,
    max_tiles: u32,
    mesh: graphics::Mesh,
    texture: graphics::Texture,
}

impl World {
    const TEXTURE_SIZE: Extent2d<u32> = Extent2d::new(8, 24);

    pub(crate) fn new(
        graphics: &graphics::Context,
        size: Extent3d<u32>,
        max_tiles: u32,
    ) -> Result<Self, WorldError> {
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
        let texture_size = graphics::texture::Size::D2(Extent2d::new(
            Self::TEXTURE_SIZE.width * max_tiles,
            Self::TEXTURE_SIZE.height,
        ));
        let texture = graphics
            .create_texture(
                texture_size,
                graphics::texture::Format::Rgba8Unorm,
                Some(graphics::texture::Sampler::new(
                    graphics::texture::FilterMode::Nearest,
                    graphics::texture::AddressMode::ClampToEdge,
                )),
                None,
            )
            .map_err(|error| WorldError::from(error))?;
        Ok(Self {
            size,
            origin_offset,
            max_tiles,
            voxels,
            mesh,
            texture,
        })
    }

    pub fn size(&self) -> Extent3d<u32> {
        self.size
    }

    pub fn mesh(&self) -> &graphics::Mesh {
        &self.mesh
    }

    pub fn texture(&self) -> &graphics::Texture {
        &self.texture
    }

    pub fn get_voxel(&self, position: Offset3d<i32>) -> Option<&Voxel> {
        self.voxels.get(self.get_voxel_index(position)?)
    }

    pub fn set_voxel(
        &mut self,
        position: Offset3d<i32>,
        voxel: Voxel,
    ) -> Result<Voxel, WorldError> {
        let Some(index) = self.get_voxel_index(position) else { return Err(WorldError::PositionInvalid(position)); };
        if let Voxel::Tile(tile_index) = voxel {
            if tile_index >= self.max_tiles {
                return Err(WorldError::TileIndexInvalid(tile_index));
            }
        }
        let old_voxel = &mut self.voxels[index];
        *old_voxel = voxel;
        Ok(*old_voxel)
    }

    pub fn set_voxel_texture(
        &self,
        tile_index: u32,
        layout: TextureLayout,
        pixels: &[u8],
    ) -> Result<(), WorldError> {
        if tile_index >= self.max_tiles {
            return Err(WorldError::TileIndexInvalid(tile_index));
        }
        let required_size = match layout {
            TextureLayout::Single => {
                (4 * Self::TEXTURE_SIZE.width * Self::TEXTURE_SIZE.height) as usize
            }
        };
        if pixels.len() < required_size {
            return Err(WorldError::DataInvalid);
        }

        let origin = Offset3d::new(Self::TEXTURE_SIZE.width * tile_index, 0, 0);
        let size = Extent3d::new(Self::TEXTURE_SIZE.width, Self::TEXTURE_SIZE.height, 1);
        self.texture.write(origin, size, pixels)?;
        Ok(())
    }

    pub fn iter(&self) -> WorldIterator<'_> {
        WorldIterator::new(self)
    }

    pub fn generate_mesh(&mut self) {
        self.mesh.vertices.clear();
        let mut staging = Vec::new();
        for (voxel_position, voxel) in self.iter() {
            match voxel {
                Voxel::Void => continue,
                Voxel::Tile(tile_index) => {
                    let world_position = Vector3::new(
                        voxel_position.x as f32,
                        voxel_position.y as f32,
                        voxel_position.z as f32,
                    );
                    staging.extend((0..CUBE_VERTEX_COUNT).into_iter().map(|vertex_index| {
                        let position = CUBE_VERTEX_POSITIONS[vertex_index] + world_position;
                        let color = Color::white();
                        let unscaled_uv = CUBE_VERTEX_UVS[vertex_index];
                        let uv = Vector2::new(
                            (*tile_index as f32 + unscaled_uv.x) / self.max_tiles as f32,
                            unscaled_uv.y,
                        );
                        graphics::mesh::Vertex::new(position, color, uv)
                    }));
                }
            }
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
    fn get_voxel_position(&self, index: usize) -> Option<Offset3d<i32>> {
        if index >= self.voxels.len() {
            None
        } else {
            unsafe { Some(self.get_voxel_position_unchecked(index)) }
        }
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
}
