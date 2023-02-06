use crate::{Extent3d, Offset3d};

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

pub struct World {
    size: Extent3d<u32>,
    origin_offset: Offset3d<i32>,
    voxels: Vec<Voxel>,
}

impl World {
    pub(crate) fn new(size: Extent3d<u32>) -> Self {
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
        Self {
            size,
            origin_offset,
            voxels,
        }
    }

    pub fn size(&self) -> Extent3d<u32> {
        self.size
    }

    pub fn get_voxel(&self, position: Offset3d<i32>) -> Option<&Voxel> {
        self.voxels.get(self.get_voxel_index(position))
    }

    pub fn set_voxel(&mut self, position: Offset3d<i32>, voxel: Voxel) -> bool {
        let index = self.get_voxel_index(position);
        let Some(old_voxel) = self.voxels.get_mut(index) else { return false; };
        *old_voxel = voxel;
        true
    }

    pub fn iter(&self) -> WorldIterator<'_> {
        WorldIterator::new(self)
    }

    #[inline]
    fn get_voxel_index(&self, position: Offset3d<i32>) -> usize {
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
