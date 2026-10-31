//! F-057. A ready brick is uploaded once, and an absent slice is not a texture.
//!
//! PS3.3 C.7.6.3 Pixel Representation is already applied by `place`. The
//! bytes here are the container. AC2's eight values are the hand-filled
//! store, x fastest, slice 0 then slice 1.
//!
//! The GPU rows are `#[ignore]`d. The tier C refusal and the stamp walk
//! need no device. `sync_ready_bricks` is the function `sync_volume` calls
//! before any stamp write.

use std::collections::BTreeMap;

use glam::DMat3;
use ocelli_cache::{BrickKey, VolumeId};
use ocelli_core::{Pt, World};
use ocelli_render::{
    BrickUploadError, ProgressError, ProgressReport, RenderGraph, Tier, sync_ready_bricks,
};
use ocelli_volume::{Level, SlicePresence, Volume, VoxelKind, VoxelStore};

#[test]
fn sync_is_unavailable_on_tier_c() {
    let mut volume = empty_volume([4, 1, 2], VoxelKind::U8);
    assert!(volume.place(0, &[1, 2, 3, 4]).is_ok());
    assert!(volume.place(1, &[5, 6, 7, 8]).is_ok());
    let mut stamps = BTreeMap::new();
    let kept = key_at(1, [1, 2, 3]);
    stamps.insert(kept, vec![4, 5]);
    let mut calls = 0u32;
    let result = sync_ready_bricks(
        Tier::Cpu,
        &mut stamps,
        VolumeId(1),
        &volume,
        |_key, _brick| {
            calls += 1;
            Ok(())
        },
    );
    assert!(matches!(
        result,
        Err(ProgressError::Upload {
            brick: None,
            source: BrickUploadError::Unavailable,
        })
    ));
    assert_eq!(calls, 0);
    assert_eq!(
        stamps.get(&kept).map(Vec::as_slice),
        Some([4, 5].as_slice())
    );
    assert_eq!(stamps.len(), 1);
    assert!(!stamps.contains_key(&key_at(1, [0, 0, 0])));
}

#[test]
fn equal_brick_bytes_are_not_uploaded_twice() {
    let mut volume = empty_volume([4, 1, 2], VoxelKind::U8);
    assert!(volume.place(0, &[1, 2, 3, 4]).is_ok());
    assert!(volume.place(1, &[5, 6, 7, 8]).is_ok());
    let mut stamps = BTreeMap::new();
    let mut calls = 0u32;
    let first = sync_ready_bricks(Tier::A, &mut stamps, VolumeId(3), &volume, |key, brick| {
        calls += 1;
        assert_eq!(key.level, 0);
        assert_eq!(key.brick, [0, 0, 0]);
        assert_eq!(brick.samples, vec![1, 2, 3, 4, 5, 6, 7, 8]);
        Ok(())
    });
    assert_eq!(
        first.ok(),
        Some(ProgressReport {
            present_slices: 2,
            slice_count: 2,
            uploaded: 1,
            skipped: 0,
        })
    );
    assert_eq!(calls, 1);
    assert_eq!(
        stamps.get(&key_at(3, [0, 0, 0])).map(Vec::as_slice),
        Some([1, 2, 3, 4, 5, 6, 7, 8].as_slice())
    );
    calls = 0;
    let second = sync_ready_bricks(
        Tier::A,
        &mut stamps,
        VolumeId(3),
        &volume,
        |_key, _brick| {
            calls += 1;
            Ok(())
        },
    );
    assert_eq!(
        second.ok(),
        Some(ProgressReport {
            present_slices: 2,
            slice_count: 2,
            uploaded: 0,
            skipped: 1,
        })
    );
    assert_eq!(calls, 0);
}

#[test]
fn replaced_brick_bytes_are_uploaded_again() {
    let mut volume = empty_volume([4, 1, 2], VoxelKind::U8);
    assert!(volume.place(0, &[1, 2, 3, 4]).is_ok());
    assert!(volume.place(1, &[5, 6, 7, 8]).is_ok());
    let mut stamps = BTreeMap::new();
    let primed = sync_ready_bricks(
        Tier::A,
        &mut stamps,
        VolumeId(4),
        &volume,
        |_key, _brick| Ok(()),
    );
    assert!(primed.is_ok());
    assert!(volume.place(0, &[9, 9, 9, 9]).is_ok());
    let mut calls = 0u32;
    let again = sync_ready_bricks(Tier::A, &mut stamps, VolumeId(4), &volume, |key, brick| {
        calls += 1;
        assert_eq!(key.level, 0);
        assert_eq!(brick.samples, vec![9, 9, 9, 9, 5, 6, 7, 8]);
        Ok(())
    });
    assert_eq!(
        again.ok(),
        Some(ProgressReport {
            present_slices: 2,
            slice_count: 2,
            uploaded: 1,
            skipped: 0,
        })
    );
    assert_eq!(calls, 1);
    assert_eq!(
        stamps.get(&key_at(4, [0, 0, 0])).map(Vec::as_slice),
        Some([9, 9, 9, 9, 5, 6, 7, 8].as_slice())
    );
}

#[test]
fn an_absent_slice_counts_as_skipped() {
    let mut volume = empty_volume([4, 1, 2], VoxelKind::U8);
    assert!(volume.place(0, &[1, 2, 3, 4]).is_ok());
    let mut stamps = BTreeMap::new();
    let mut calls = 0u32;
    let synced = sync_ready_bricks(
        Tier::A,
        &mut stamps,
        VolumeId(5),
        &volume,
        |_key, _brick| {
            calls += 1;
            Ok(())
        },
    );
    assert_eq!(
        synced.ok(),
        Some(ProgressReport {
            present_slices: 1,
            slice_count: 2,
            uploaded: 0,
            skipped: 1,
        })
    );
    assert_eq!(calls, 0);
    assert!(stamps.is_empty());
}

#[test]
fn a_failed_later_brick_keeps_the_earlier_stamp() {
    let mut volume = empty_volume([4, 1, 129], VoxelKind::U8);
    let mut n = 0u32;
    while n < 129 {
        assert!(volume.place(n, &[1, 2, 3, 4]).is_ok());
        n += 1;
    }
    let mut stamps = BTreeMap::new();
    let result = sync_ready_bricks(Tier::A, &mut stamps, VolumeId(9), &volume, |key, brick| {
        if key.brick == [0, 0, 0] {
            let [_, _, depth] = brick.extent;
            assert_eq!(depth, 128);
            Ok(())
        } else {
            Err(ProgressError::Upload {
                brick: Some(brick),
                source: BrickUploadError::FormatUnavailable,
            })
        }
    });
    assert!(matches!(
        result,
        Err(ProgressError::Upload {
            brick: Some(_),
            source: BrickUploadError::FormatUnavailable,
        })
    ));
    assert!(stamps.contains_key(&key_at(9, [0, 0, 0])));
    assert!(!stamps.contains_key(&key_at(9, [0, 0, 1])));
    assert_eq!(stamps.len(), 1);
}

#[test]
fn a_grid_product_that_overflows_is_refused() {
    let volume = overflow_volume();
    let mut stamps = BTreeMap::new();
    let mut calls = 0u32;
    let result = sync_ready_bricks(
        Tier::A,
        &mut stamps,
        VolumeId(8),
        &volume,
        |_key, _brick| {
            calls += 1;
            Ok(())
        },
    );
    assert!(matches!(result, Err(ProgressError::Grid)));
    assert_eq!(calls, 0);
    assert!(stamps.is_empty());
}

fn key_at(id: u64, brick: [u32; 3]) -> BrickKey {
    BrickKey {
        volume: VolumeId(id),
        level: 0,
        brick,
    }
}

fn empty_volume(dims: [u32; 3], voxel: VoxelKind) -> Volume {
    Volume {
        dims,
        spacing: [1.0, 1.0, 1.0],
        origin: Pt::<World>::new(0.0, 0.0, 0.0),
        direction: DMat3::IDENTITY,
        voxel,
        data: VoxelStore::new(dims, voxel),
        levels: vec![Level {
            dims,
            spacing: [1.0, 1.0, 1.0],
            offset: 0,
        }],
        present: SlicePresence::new(dims[2]),
        order: Vec::new(),
    }
}

fn overflow_volume() -> Volume {
    let dims = [u32::MAX, u32::MAX, 1];
    Volume {
        dims,
        spacing: [1.0, 1.0, 1.0],
        origin: Pt::<World>::new(0.0, 0.0, 0.0),
        direction: DMat3::IDENTITY,
        voxel: VoxelKind::U8,
        data: VoxelStore::new([1, 1, 2], VoxelKind::U8),
        levels: vec![Level {
            dims,
            spacing: [1.0, 1.0, 1.0],
            offset: 0,
        }],
        present: SlicePresence::new(1),
        order: Vec::new(),
    }
}

fn native_clock() -> impl FnMut() -> u64 {
    let start = std::time::Instant::now();
    move || u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn session() -> Option<(ocelli_render::ResolvedAdapter, ocelli_render::GpuContext)> {
    let mut clock = native_clock();
    let adapter = pollster::block_on(ocelli_render::resolve_adapter(
        ocelli_render::TierRequest::Auto,
        &mut clock,
    ))?;
    let context = pollster::block_on(adapter.open()).ok()?;
    Some((adapter, context))
}

fn require_graph() -> Option<(
    ocelli_render::ResolvedAdapter,
    ocelli_render::GpuContext,
    RenderGraph,
)> {
    let (adapter, mut context) = session()?;
    match RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba32Float) {
        Ok(graph) => Some((adapter, context, graph)),
        Err(_) => unreachable!("a tier A or B context opens a graph"),
    }
}

fn voxel_offset(x: usize, y: usize, z: usize, height: usize) -> Option<usize> {
    let stride = 256usize;
    let row = y.checked_mul(stride)?;
    let plane = stride.checked_mul(height)?.checked_mul(z)?;
    plane.checked_add(row)?.checked_add(x)
}

fn unpack_u8(copied: &[u8], extent: [u32; 3]) -> Vec<u8> {
    let [width, height, depth] = extent;
    let width_us = usize::try_from(width).unwrap_or(0);
    let height_us = usize::try_from(height).unwrap_or(0);
    let depth_us = usize::try_from(depth).unwrap_or(0);
    let mut out = Vec::new();
    let mut z = 0usize;
    while z < depth_us {
        let mut y = 0usize;
        while y < height_us {
            let mut x = 0usize;
            while x < width_us {
                let Some(offset) = voxel_offset(x, y, z, height_us) else {
                    return Vec::new();
                };
                let Some(byte) = copied.get(offset).copied() else {
                    return Vec::new();
                };
                out.push(byte);
                x += 1;
            }
            y += 1;
        }
        z += 1;
    }
    out
}

fn read_u8(
    context: &ocelli_render::GpuContext,
    texture: &wgpu::Texture,
    extent: [u32; 3],
) -> Vec<u8> {
    let [width, height, depth] = extent;
    let Some(size) = 256u64
        .checked_mul(u64::from(height))
        .and_then(|rows| rows.checked_mul(u64::from(depth)))
    else {
        return Vec::new();
    };
    let device = context.device();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("progressive brick readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("progressive brick readback"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: depth,
        },
    );
    context.queue().submit(std::iter::once(encoder.finish()));
    let slice = buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    let _ = device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    });
    let _ = receiver.recv();
    let copied = {
        let Ok(data) = slice.get_mapped_range() else {
            return Vec::new();
        };
        data.to_vec()
    };
    buffer.unmap();
    unpack_u8(&copied, extent)
}

fn read_resident(
    graph: &RenderGraph,
    context: &ocelli_render::GpuContext,
    key: &BrickKey,
    extent: [u32; 3],
    format: wgpu::TextureFormat,
) -> Vec<u8> {
    assert_eq!(graph.brick_extent(key), Some(extent));
    assert!(graph.brick_texture(key).is_some());
    let Some(texture) = graph.brick_texture(key) else {
        return Vec::new();
    };
    assert_eq!(texture.dimension(), wgpu::TextureDimension::D3);
    assert_eq!(texture.format(), format);
    let [width, height, depth] = extent;
    assert_eq!(
        texture.size(),
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: depth,
        }
    );
    assert!(texture.usage().contains(wgpu::TextureUsages::COPY_SRC));
    read_u8(context, texture, extent)
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn sync_uploads_a_ready_brick_once() {
    let Some((adapter, context, mut graph)) = require_graph() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let mut volume = empty_volume([4, 1, 2], VoxelKind::U8);
    assert!(volume.place(0, &[1, 2, 3, 4]).is_ok());
    assert!(volume.place(1, &[5, 6, 7, 8]).is_ok());
    let before = graph.brick_uploads();
    let synced = graph.sync_volume(&context, &adapter, VolumeId(7), &volume);
    let Ok(report) = synced else {
        unreachable!("both slices of the only brick have landed")
    };
    assert_eq!(report.present_slices, 2);
    assert_eq!(report.slice_count, 2);
    assert_eq!(report.uploaded, 1);
    assert_eq!(report.skipped, 0);
    assert_eq!(graph.brick_uploads(), before + 1);
    let key = key_at(7, [0, 0, 0]);
    assert_eq!(
        read_resident(
            &graph,
            &context,
            &key,
            [4, 1, 2],
            wgpu::TextureFormat::R8Uint
        ),
        vec![1, 2, 3, 4, 5, 6, 7, 8]
    );
    let again = graph.sync_volume(&context, &adapter, VolumeId(7), &volume);
    let Ok(again) = again else {
        unreachable!("the second sync of equal bytes is not a refusal")
    };
    assert_eq!(again.uploaded, 0);
    assert_eq!(again.skipped, 1);
    assert_eq!(graph.brick_uploads(), before + 1);

    let recovered = ocelli_render::Recovered {
        caps: *context.caps(),
        graph: Some(ocelli_render::GraphResources {
            pipelines_restored: 1,
            cache_generation: 2,
        }),
    };
    graph.install_recovered(&context, &recovered);
    assert!(graph.brick_texture(&key).is_none());
    let restored = graph.sync_volume(&context, &adapter, VolumeId(7), &volume);
    let Ok(restored) = restored else {
        unreachable!("recovery leaves the tier that opened the graph")
    };
    assert_eq!(restored.uploaded, 1);
    assert_eq!(graph.brick_uploads(), before + 2);
    assert_eq!(
        read_resident(
            &graph,
            &context,
            &key,
            [4, 1, 2],
            wgpu::TextureFormat::R8Uint
        ),
        vec![1, 2, 3, 4, 5, 6, 7, 8]
    );
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn sync_skips_a_brick_with_an_absent_slice() {
    let Some((adapter, context, mut graph)) = require_graph() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let mut volume = empty_volume([4, 1, 2], VoxelKind::U8);
    assert!(volume.place(0, &[1, 2, 3, 4]).is_ok());
    let before = graph.brick_uploads();
    let synced = graph.sync_volume(&context, &adapter, VolumeId(7), &volume);
    let Ok(report) = synced else {
        unreachable!("an absent slice is a skip, not an upload error")
    };
    assert_eq!(report.present_slices, 1);
    assert_eq!(report.slice_count, 2);
    assert_eq!(report.uploaded, 0);
    assert_eq!(report.skipped, 1);
    assert_eq!(graph.brick_uploads(), before);
    assert!(graph.brick_texture(&key_at(7, [0, 0, 0])).is_none());
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn a_replaced_slice_uploads_again() {
    let Some((adapter, context, mut graph)) = require_graph() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let mut volume = empty_volume([4, 1, 2], VoxelKind::U8);
    assert!(volume.place(0, &[1, 2, 3, 4]).is_ok());
    assert!(volume.place(1, &[5, 6, 7, 8]).is_ok());
    let before = graph.brick_uploads();
    let primed = graph.sync_volume(&context, &adapter, VolumeId(7), &volume);
    assert!(primed.is_ok());
    assert_eq!(graph.brick_uploads(), before + 1);
    assert!(volume.place(0, &[9, 9, 9, 9]).is_ok());
    let synced = graph.sync_volume(&context, &adapter, VolumeId(7), &volume);
    let Ok(report) = synced else {
        unreachable!("a replaced present slice is still a ready brick")
    };
    assert_eq!(report.uploaded, 1);
    assert_eq!(report.skipped, 0);
    assert_eq!(graph.brick_uploads(), before + 2);
    assert_eq!(
        read_resident(
            &graph,
            &context,
            &key_at(7, [0, 0, 0]),
            [4, 1, 2],
            wgpu::TextureFormat::R8Uint
        ),
        vec![9, 9, 9, 9, 5, 6, 7, 8]
    );
}
