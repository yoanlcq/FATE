use std::path::PathBuf;
use std::io;
use fate::math::{Rgb, Rgba};
use fate::mt;
use fate::img;
use viewport::ViewportNode;
use eid::EID;
use cubemap::{CubemapSelector, CubemapArrayID, CubemapArrayInfo, CubemapFace};
use texture2d::{Texture2DArrayID, Texture2DArrayInfo};
use gpu::{GpuTextureInternalFormat, CpuSubImage2D, CpuImgFormat, CpuImgPixelType, CpuPixels, GpuTextureFilter};
use system::*;

mod cubemap {
    use super::*;
    pub const RGB8_1L_1X1: CubemapArrayID = CubemapArrayID(0);
    pub const RGB8_1L_1024X1024: CubemapArrayID = CubemapArrayID(1);
}

mod texture2d {
    use super::*;
    pub const RGB8_1L_1X1: Texture2DArrayID = Texture2DArrayID(0);
    pub const RGB8_1L_1024X1024: Texture2DArrayID = Texture2DArrayID(1);
    pub const RGB8_1L_256X256: Texture2DArrayID = Texture2DArrayID(2);
}


type ImgFuture = mt::Future<mt::Then<mt::ReadFile, mt::Async<io::Result<img::Result<(img::Metadata, img::AnyImage)>>>>>;

#[derive(Debug)]
struct CubemapFaceRequest {
    future: Option<ImgFuture>,
    path: PathBuf,
    array_id: CubemapArrayID,
    cubemap_index: u32,
    face: CubemapFace,
}

#[derive(Debug)]
struct Texture2DRequest {
    future: Option<ImgFuture>,
    path: PathBuf,
    array_id: Texture2DArrayID,
    slot: u32,
}

#[derive(Debug)]
pub struct Gameplay {
    cubemap_face_requests: Vec<CubemapFaceRequest>,
    texture2d_requests: Vec<Texture2DRequest>,
}

fn format_mem(b: usize) -> String {
    let kb = b / 1024;
    if kb == 0 { return format!("{} b", b); }
    let mib = kb / 1024;
    if mib == 0 { return format!("{} Kb", kb); }
    let gib = mib / 1024;
    if gib == 0 { return format!("{} MiB", mib); }
    
    format!("{} GiB", gib)
}

impl Gameplay {
    pub fn new(g: &mut G) -> Self {
        {
            let mut leaf = g.viewport_db_mut().root_node().value.unwrap_leaf().borrow_mut();
            leaf.skybox_cubemap_selector = Some(CubemapSelector { array_id: cubemap::RGB8_1L_1024X1024, cubemap: 0, });
        }

        let cubemap_array_infos = [
            (cubemap::RGB8_1L_1X1, CubemapArrayInfo { nb_levels: 1, internal_format: GpuTextureInternalFormat::RGB8, size: Extent2::one(), nb_cubemaps: 16, }),
            (cubemap::RGB8_1L_1024X1024, CubemapArrayInfo { nb_levels: 1, internal_format: GpuTextureInternalFormat::RGB8, size: Extent2::broadcast(1024), nb_cubemaps: 6, }),
        ];
        let texture2d_array_infos = [
            (texture2d::RGB8_1L_1X1, Texture2DArrayInfo { nb_levels: 1, internal_format: GpuTextureInternalFormat::RGB8, size: Extent2::one(), nb_slots: 2, }),
            (texture2d::RGB8_1L_256X256, Texture2DArrayInfo { nb_levels: 1, internal_format: GpuTextureInternalFormat::RGB8, size: Extent2::broadcast(256), nb_slots: 3, }),
            (texture2d::RGB8_1L_1024X1024, Texture2DArrayInfo { nb_levels: 1, internal_format: GpuTextureInternalFormat::RGB8, size: Extent2::broadcast(1024), nb_slots: 2, }),
        ];


        let mut tex_mem = 0;

        for (array_id, info) in cubemap_array_infos.iter() {
            tex_mem += info.memory_usage();
            info!("Memory usage of {:?}: {}", array_id, format_mem(info.memory_usage()));
            g.cubemap_array_create(*array_id, *info);
        }
        for (array_id, info) in texture2d_array_infos.iter() {
            tex_mem += info.memory_usage();
            info!("Memory usage of {:?}: {}", array_id, format_mem(info.memory_usage()));
            g.texture2d_array_create(*array_id, *info);
        }

        // Use max. 512 Mib total on the GPU
        let max_mem = 512 * 1024 * 1024;
        // Max. 2 Mib of scratch space (misc unpredictable allocations)
        let scratch_mem = 2 * 1024 * 1024;

        // 432 Mib
        let max_chunks = 3*3*3;
        let chunk_mem = 8 * 1024 * 1024;

        info!("tex_mem         : {}", format_mem(tex_mem));
        info!("scratch_mem     : {}", format_mem(scratch_mem));
        info!("total_chunks_mem: {}", format_mem(max_chunks * chunk_mem));
        info!("max_mem         : {}", format_mem(max_mem));
        assert!(tex_mem + scratch_mem + max_chunks * chunk_mem <= max_mem);

        fn pixel(rgb: Rgb<u8>) -> CpuSubImage2D {
            CpuSubImage2D::from_rgb_u8_pixel(rgb)
        }

        // TODO:
        // GL_TEXTURE_MAX_ANISOTROPY GL_MAX_TEXTURE_MAX_ANISOTROPY GL_LINEAR_MIPMAP_LINEAR
        // ARB_texture_filter_anisotropic EXT_texture_filter_anisotropic
        g.cubemap_array_clear(cubemap::RGB8_1L_1X1, 0, Rgba::magenta());

        g.cubemap_array_set_min_filter(cubemap::RGB8_1L_1X1, GpuTextureFilter::Nearest);
        g.cubemap_array_set_mag_filter(cubemap::RGB8_1L_1X1, GpuTextureFilter::Nearest);

        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 0, CubemapFace::PositiveX, pixel(Rgb::new(000, 000, 000)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 0, CubemapFace::NegativeX, pixel(Rgb::new(000, 000, 000)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 0, CubemapFace::PositiveY, pixel(Rgb::new(000, 000, 000)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 0, CubemapFace::NegativeY, pixel(Rgb::new(000, 000, 000)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 0, CubemapFace::PositiveZ, pixel(Rgb::new(000, 000, 000)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 0, CubemapFace::NegativeZ, pixel(Rgb::new(000, 000, 000)));

        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 1, CubemapFace::PositiveX, pixel(Rgb::new(255, 255, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 1, CubemapFace::NegativeX, pixel(Rgb::new(255, 255, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 1, CubemapFace::PositiveY, pixel(Rgb::new(255, 255, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 1, CubemapFace::NegativeY, pixel(Rgb::new(255, 255, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 1, CubemapFace::PositiveZ, pixel(Rgb::new(255, 255, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 1, CubemapFace::NegativeZ, pixel(Rgb::new(255, 255, 255)));

        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 2, CubemapFace::PositiveX, pixel(Rgb::new(255, 000, 000)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 2, CubemapFace::NegativeX, pixel(Rgb::new(000, 255, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 2, CubemapFace::PositiveY, pixel(Rgb::new(000, 255, 000)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 2, CubemapFace::NegativeY, pixel(Rgb::new(255, 000, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 2, CubemapFace::PositiveZ, pixel(Rgb::new(000, 000, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 2, CubemapFace::NegativeZ, pixel(Rgb::new(255, 255, 000)));

        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 3, CubemapFace::PositiveX, pixel(Rgb::new(000, 255, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 3, CubemapFace::NegativeX, pixel(Rgb::new(000, 255, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 3, CubemapFace::PositiveY, pixel(Rgb::new(000, 000, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 3, CubemapFace::NegativeY, pixel(Rgb::new(255, 255, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 3, CubemapFace::PositiveZ, pixel(Rgb::new(000, 255, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 3, CubemapFace::NegativeZ, pixel(Rgb::new(000, 255, 255)));

        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 4, CubemapFace::PositiveX, pixel(Rgb::new(255, 175,  45)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 4, CubemapFace::NegativeX, pixel(Rgb::new(255, 175,  45)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 4, CubemapFace::PositiveY, pixel(Rgb::new(255, 000, 000)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 4, CubemapFace::NegativeY, pixel(Rgb::new(255, 255, 000)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 4, CubemapFace::PositiveZ, pixel(Rgb::new(255, 175,  45)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 4, CubemapFace::NegativeZ, pixel(Rgb::new(255, 175,  45)));

        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 5, CubemapFace::PositiveX, pixel(Rgb::new(255, 255, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 5, CubemapFace::NegativeX, pixel(Rgb::new(255, 255, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 5, CubemapFace::PositiveY, pixel(Rgb::new(255, 255, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 5, CubemapFace::NegativeY, pixel(Rgb::new(255, 255, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 5, CubemapFace::PositiveZ, pixel(Rgb::new(255, 255, 255)));
        g.cubemap_array_sub_image_2d(cubemap::RGB8_1L_1X1, 5, CubemapFace::NegativeZ, pixel(Rgb::new(255, 255, 255)));

        g.cubemap_array_clear(cubemap::RGB8_1L_1024X1024, 0, Rgba::magenta());

        g.cubemap_array_set_min_filter(cubemap::RGB8_1L_1024X1024, GpuTextureFilter::Linear);
        g.cubemap_array_set_mag_filter(cubemap::RGB8_1L_1024X1024, GpuTextureFilter::Linear);


        g.texture2d_array_clear(texture2d::RGB8_1L_1X1, 0, Rgba::cyan());
        g.texture2d_array_clear(texture2d::RGB8_1L_256X256, 0, Rgba::cyan());
        g.texture2d_array_clear(texture2d::RGB8_1L_1024X1024, 0, Rgba::cyan());

        g.texture2d_array_set_min_filter(texture2d::RGB8_1L_1X1, GpuTextureFilter::Nearest);
        g.texture2d_array_set_mag_filter(texture2d::RGB8_1L_1X1, GpuTextureFilter::Nearest);
        g.texture2d_array_set_min_filter(texture2d::RGB8_1L_256X256, GpuTextureFilter::Linear);
        g.texture2d_array_set_min_filter(texture2d::RGB8_1L_256X256, GpuTextureFilter::Linear);
        g.texture2d_array_set_min_filter(texture2d::RGB8_1L_1024X1024, GpuTextureFilter::Linear);
        g.texture2d_array_set_mag_filter(texture2d::RGB8_1L_1024X1024, GpuTextureFilter::Linear);

        g.texture2d_array_sub_image_2d(texture2d::RGB8_1L_1X1, 0, pixel(Rgb::new(000, 000, 000)));
        g.texture2d_array_sub_image_2d(texture2d::RGB8_1L_1X1, 1, pixel(Rgb::new(255, 255, 255)));


        // Upload cubemap textures (async)
        
        let dir = g.res.data_path().join(PathBuf::from("art/3rdparty/mayhem"));
        let suffixes = CubemapFace::TERRAGEN_SUFFIXES;
        let extension = "jpg";
        let mut cubemap_face_requests = vec![];
        for (cubemap_index, name) in ["grouse", "aqua4", "h2s", "flame"].iter().enumerate() {
            for suffix in suffixes.iter() {
                cubemap_face_requests.push(CubemapFaceRequest {
                    path: dir.join(format!("{}_{}.{}", name, suffix, extension)),
                    array_id: cubemap::RGB8_1L_1024X1024,
                    cubemap_index: cubemap_index as _,
                    face: CubemapFace::try_from_terragen_suffix(suffix).unwrap(),
                    future: None,
                });
            }
        }

        let dir = g.res.data_path().join(PathBuf::from("art/tex2d"));
        let mut texture2d_requests = vec![];
        for (i, name) in ["maze.png", "plasma.png", "checkerboard.png"].iter().enumerate() {
            texture2d_requests.push(Texture2DRequest {
                path: dir.join(name),
                array_id: texture2d::RGB8_1L_256X256,
                slot: i as _,
                future: None,
            });
        }

        for req in cubemap_face_requests.iter_mut() {
            use self::mt::TaskExt;
            let future = g.mt.schedule(mt::ReadFile::new(&req.path).then(|result: io::Result<Vec<u8>>| {
                mt::Async::new(move || result.map(|data| img::load_from_memory(data)))
            }));
            req.future = Some(future);
        }

        for req in texture2d_requests.iter_mut() {
            use self::mt::TaskExt;
            let future = g.mt.schedule(mt::ReadFile::new(&req.path).then(|result: io::Result<Vec<u8>>| {
                mt::Async::new(move || result.map(|data| img::load_from_memory(data)))
            }));
            req.future = Some(future);
        }

        // TODO: Upload font atlas
        
        Gameplay {
            cubemap_face_requests,
            texture2d_requests,
        }
    }
}

impl Gameplay {
    fn pump_cubemap_faces(&mut self, g: &mut G) {
        loop {
            let mut complete = None;

            for (i, req) in self.cubemap_face_requests.iter().enumerate() {
                let future = req.future.as_ref().unwrap();
                if future.is_complete() {
                    complete = Some(i);
                    break;
                }

                let _progress = match future.poll() {
                    mt::Either::Left(fp) => format!("{}%", if fp.nsize == 0 { 0. } else { fp.nread as f32 / fp.nsize as f32 }),
                    mt::Either::Right(_) => format!("Converting..."),
                };
                // text += &format!("Loading {} (z = {}): {}\n", future.as_ref().first().path().display(), z, progress);
            }

            match complete {
                None => break,
                Some(i) => {
                    let mut req = self.cubemap_face_requests.remove(i);
                    match req.future.take().unwrap().wait() {
                        Ok(Ok((_, img))) => {
                            g.cubemap_array_sub_image_2d(req.array_id, req.cubemap_index as _, req.face, CpuSubImage2D::from_any_image(img));
                            info!("Loaded `{}`", req.path.display());
                        },
                        _ => unimplemented!{},
                    }
                }
            }
        }
    }
    fn pump_texture2ds(&mut self, g: &mut G) {
        loop {
            let mut complete = None;

            for (i, req) in self.texture2d_requests.iter().enumerate() {
                let future = req.future.as_ref().unwrap();
                if future.is_complete() {
                    complete = Some(i);
                    break;
                }

                let _progress = match future.poll() {
                    mt::Either::Left(fp) => format!("{}%", if fp.nsize == 0 { 0. } else { fp.nread as f32 / fp.nsize as f32 }),
                    mt::Either::Right(_) => format!("Converting..."),
                };
                // text += &format!("Loading {} (z = {}): {}\n", future.as_ref().first().path().display(), z, progress);
            }

            match complete {
                None => break,
                Some(i) => {
                    let mut req = self.texture2d_requests.remove(i);
                    match req.future.take().unwrap().wait() {
                        Ok(Ok((_, img))) => {
                            g.texture2d_array_sub_image_2d(req.array_id, req.slot as _, CpuSubImage2D::from_any_image(img));
                            info!("Loaded `{}`", req.path.display());
                        },
                        _ => unimplemented!{},
                    }
                }
            }
        }
    }
}

impl System for Gameplay {
    fn draw(&mut self, g: &mut G, _: &Draw) {
        self.pump_cubemap_faces(g);
        self.pump_texture2ds(g);
    }
}
