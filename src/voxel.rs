use crate::{graphics, impl_from_error, Color, Extent2d, Extent3d, Offset3d, Vector2, Vector3};
use bitflags::bitflags;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Voxel {
    #[default]
    Void,
    Tile(u32),
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
enum Face {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl Face {
    fn from_index(index: u8) -> Self {
        match index {
            0 => Self::PosX,
            1 => Self::NegX,
            2 => Self::PosY,
            3 => Self::NegY,
            4 => Self::PosZ,
            5 => Self::NegZ,
            _ => unimplemented!(),
        }
    }
}

impl Face {
    const MAX_COUNT: u8 = 6;

    fn opposite(&self) -> Self {
        match self {
            Self::PosX => Self::NegX,
            Self::NegX => Self::PosX,
            Self::PosY => Self::NegY,
            Self::NegY => Self::PosY,
            Self::PosZ => Self::NegZ,
            Self::NegZ => Self::PosZ,
        }
    }

    fn get_vertex_positions(&self) -> [Vector3<f32>; 6] {
        match *self {
            Face::PosX => [
                Vector3::new(0.5, -0.5, 0.5),
                Vector3::new(0.5, 0.5, 0.5),
                Vector3::new(0.5, -0.5, -0.5),
                Vector3::new(0.5, 0.5, 0.5),
                Vector3::new(0.5, 0.5, -0.5),
                Vector3::new(0.5, -0.5, -0.5),
            ],
            Face::NegX => [
                Vector3::new(-0.5, -0.5, -0.5),
                Vector3::new(-0.5, 0.5, -0.5),
                Vector3::new(-0.5, -0.5, 0.5),
                Vector3::new(-0.5, 0.5, -0.5),
                Vector3::new(-0.5, 0.5, 0.5),
                Vector3::new(-0.5, -0.5, 0.5),
            ],
            Face::PosY => [
                Vector3::new(-0.5, 0.5, 0.5),
                Vector3::new(-0.5, 0.5, -0.5),
                Vector3::new(0.5, 0.5, 0.5),
                Vector3::new(-0.5, 0.5, -0.5),
                Vector3::new(0.5, 0.5, -0.5),
                Vector3::new(0.5, 0.5, 0.5),
            ],
            Face::NegY => [
                Vector3::new(-0.5, -0.5, -0.5),
                Vector3::new(-0.5, -0.5, 0.5),
                Vector3::new(0.5, -0.5, -0.5),
                Vector3::new(-0.5, -0.5, 0.5),
                Vector3::new(0.5, -0.5, 0.5),
                Vector3::new(0.5, -0.5, -0.5),
            ],
            Face::PosZ => [
                Vector3::new(-0.5, -0.5, 0.5),
                Vector3::new(-0.5, 0.5, 0.5),
                Vector3::new(0.5, -0.5, 0.5),
                Vector3::new(-0.5, 0.5, 0.5),
                Vector3::new(0.5, 0.5, 0.5),
                Vector3::new(0.5, -0.5, 0.5),
            ],
            Face::NegZ => [
                Vector3::new(0.5, -0.5, -0.5),
                Vector3::new(0.5, 0.5, -0.5),
                Vector3::new(-0.5, -0.5, -0.5),
                Vector3::new(0.5, 0.5, -0.5),
                Vector3::new(-0.5, 0.5, -0.5),
                Vector3::new(-0.5, -0.5, -0.5),
            ],
        }
    }

    fn get_vertex_uvs(&self) -> [Vector2<f32>; 6] {
        match self {
            Face::PosX | Face::NegX | Face::PosZ | Face::NegZ => [
                Vector2::new(0.0, 0.667),
                Vector2::new(0.0, 0.333),
                Vector2::new(1.0, 0.667),
                Vector2::new(0.0, 0.333),
                Vector2::new(1.0, 0.333),
                Vector2::new(1.0, 0.667),
            ],
            Face::PosY => [
                Vector2::new(0.0, 0.333),
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 0.333),
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 0.0),
                Vector2::new(1.0, 0.333),
            ],
            Face::NegY => [
                Vector2::new(0.0, 1.0),
                Vector2::new(0.0, 0.667),
                Vector2::new(1.0, 1.0),
                Vector2::new(0.0, 0.667),
                Vector2::new(1.0, 0.667),
                Vector2::new(1.0, 1.0),
            ],
        }
    }

    fn get_adjacent_voxel_position(&self) -> Offset3d<i32> {
        match *self {
            Face::PosX => Offset3d::new(1, 0, 0),
            Face::NegX => Offset3d::new(-1, 0, 0),
            Face::PosY => Offset3d::new(0, 1, 0),
            Face::NegY => Offset3d::new(0, -1, 0),
            Face::PosZ => Offset3d::new(0, 0, 1),
            Face::NegZ => Offset3d::new(0, 0, -1),
        }
    }
}

bitflags! {
    #[repr(transparent)]
    struct Faces: u8 {
        const POS_X = (1 << 0);
        const NEG_X = (1 << 1);
        const POS_Y = (1 << 2);
        const NEG_Y = (1 << 3);
        const POS_Z = (1 << 4);
        const NEG_Z = (1 << 5);
    }
}

impl From<Face> for Faces {
    fn from(value: Face) -> Self {
        match value {
            Face::PosX => Self::POS_X,
            Face::NegX => Self::NEG_X,
            Face::PosY => Self::POS_Y,
            Face::NegY => Self::NEG_Y,
            Face::PosZ => Self::POS_Z,
            Face::NegZ => Self::NEG_Z,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct VoxelData {
    voxel: Voxel,
    faces: Faces,
}

impl Default for VoxelData {
    fn default() -> Self {
        Self {
            voxel: Default::default(),
            faces: Faces::empty(),
        }
    }
}

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
    voxel_data: Vec<VoxelData>,
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
        let voxel_data =
            vec![VoxelData::default(); (size.width * size.height * size.depth) as usize];
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
            voxel_data,
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
        self.get_voxel_data(position)
            .map(|voxel_data| &voxel_data.voxel)
    }

    pub fn set_voxel(
        &mut self,
        position: Offset3d<i32>,
        voxel: Voxel,
    ) -> Result<Voxel, WorldError> {
        let Some(target_index) = self.get_voxel_index(position) else { return Err(WorldError::PositionInvalid(position)); };
        if let Voxel::Tile(tile_index) = voxel {
            if tile_index >= self.max_tiles {
                return Err(WorldError::TileIndexInvalid(tile_index));
            }
        }

        // set voxel
        let old_voxel = self.voxel_data[target_index].voxel;
        self.voxel_data[target_index].voxel = voxel;

        // set faces
        // iterate all target faces
        for face_index in 0..Face::MAX_COUNT {
            let face = Face::from_index(face_index);

            // get voxel adjacent to face
            let other_position = {
                let adjacent = face.get_adjacent_voxel_position();
                Offset3d::new(
                    position.x + adjacent.x,
                    position.y + adjacent.y,
                    position.z + adjacent.z,
                )
            };
            let Some(other_index) = self.get_voxel_index(other_position) else {
                self.voxel_data[target_index].faces.insert(Faces::from(face));
                continue;
            };
            let target_voxel = voxel;
            let other_voxel = self.voxel_data[other_index].voxel;
            match (target_voxel, other_voxel) {
                (Voxel::Void, Voxel::Void) => {}
                (Voxel::Void, Voxel::Tile(_)) => {
                    self.voxel_data[other_index]
                        .faces
                        .insert(Faces::from(face.opposite()));
                }
                (Voxel::Tile(_), Voxel::Void) => {
                    self.voxel_data[target_index]
                        .faces
                        .insert(Faces::from(face));
                }
                (Voxel::Tile(_), Voxel::Tile(_)) => {
                    self.voxel_data[other_index]
                        .faces
                        .remove(Faces::from(face.opposite()));
                }
            }
        }

        Ok(old_voxel)
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

    pub fn generate_mesh(&mut self) {
        self.mesh.vertices.clear();
        let mut staging = Vec::new();
        for (voxel_position, voxel_data) in self.iter() {
            match voxel_data.voxel {
                Voxel::Void => continue,
                Voxel::Tile(tile_index) => {
                    let world_position = Vector3::new(
                        voxel_position.x as f32,
                        voxel_position.y as f32,
                        voxel_position.z as f32,
                    );
                    for face_index in 0..Face::MAX_COUNT {
                        let face = Face::from_index(face_index);
                        if !voxel_data.faces.contains(Faces::from(face)) {
                            continue;
                        }
                        let vertex_positions = face.get_vertex_positions();
                        let vertex_uvs = face.get_vertex_uvs();
                        staging.extend(vertex_positions.iter().zip(vertex_uvs.iter()).map(
                            |(position, uv)| {
                                let position = *position + world_position;
                                let color = Color::white();
                                let uv = Vector2::new(
                                    (tile_index as f32 + uv.x) / self.max_tiles as f32,
                                    uv.y,
                                );
                                graphics::mesh::Vertex::new(position, color, uv)
                            },
                        ));
                    }
                }
            }
        }
        self.mesh.vertices.extend_from_slice(&staging);
    }

    #[inline]
    fn iter(&self) -> WorldIterator<'_> {
        WorldIterator::new(self)
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
        if index >= self.voxel_data.len() {
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

    #[inline]
    fn get_voxel_data(&self, position: Offset3d<i32>) -> Option<&VoxelData> {
        self.voxel_data.get(self.get_voxel_index(position)?)
    }
}

struct WorldIterator<'a> {
    index: usize,
    world: &'a World,
}

impl<'a> WorldIterator<'a> {
    fn new(world: &'a World) -> Self {
        Self { index: 0, world }
    }
}

impl<'a> Iterator for WorldIterator<'a> {
    type Item = (Offset3d<i32>, &'a VoxelData);

    fn next(&mut self) -> Option<Self::Item> {
        let position = self.world.get_voxel_position(self.index)?;
        self.index += 1;
        let voxel_data = self.world.get_voxel_data(position)?;
        Some((position, voxel_data))
    }
}
