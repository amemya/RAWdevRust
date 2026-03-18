use std::io::{BufWriter, Write};
use std::path::Path;
use little_exif::metadata::Metadata;
use little_exif::exif_tag::ExifTag;
use little_exif::rational::uR64;

/// RGBRGB... バッファを PPM ファイルとして保存
pub fn save_ppm(rgb: &[u8], width: usize, height: usize, path: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut writer = BufWriter::new(file);
    write!(writer, "P6\n{} {}\n255\n", width, height)?;
    writer.write_all(rgb)?;
    Ok(())
}

/// RGBRGB... バッファを PNG ファイルとして保存 (PNG-24: 8-bit RGB)
pub fn save_png(
    rgb: &[u8],
    width: usize,
    height: usize,
    path: &Path,
    exif_info: &crate::decode::ExifInfo,
    color_space: crate::color::TargetColorSpace,
) -> anyhow::Result<()> {
    let mut metadata = Metadata::new();
    
    if let Some(make) = &exif_info.make {
        metadata.set_tag(ExifTag::Make(make.clone()));
    }
    if let Some(model) = &exif_info.model {
        metadata.set_tag(ExifTag::Model(model.clone()));
    }
    if let Some(dt) = &exif_info.datetime {
        metadata.set_tag(ExifTag::DateTimeOriginal(dt.clone()));
    }
    if let Some(iso) = exif_info.iso {
        metadata.set_tag(ExifTag::ISO(vec![iso.min(u16::MAX as u32) as u16]));
    }
    if let Some((n, d)) = exif_info.f_number {
        metadata.set_tag(ExifTag::FNumber(vec![uR64 { nominator: n, denominator: d }]));
    }
    if let Some((n, d)) = exif_info.exposure_time {
        metadata.set_tag(ExifTag::ExposureTime(vec![uR64 { nominator: n, denominator: d }]));
    }
    if let Some((n, d)) = exif_info.focal_length {
        metadata.set_tag(ExifTag::FocalLength(vec![uR64 { nominator: n, denominator: d }]));
    }
    if let Some(lens) = &exif_info.lens_model {
        metadata.set_tag(ExifTag::LensModel(lens.clone()));
    }

    let encoded_metadata = metadata.encode().map_err(|e| anyhow::anyhow!("Failed to encode EXIF: {:?}", e))?;

    let file = std::fs::File::create(path)?;
    let mut buf_writer = BufWriter::new(file);

    let mut encoder = png::Encoder::new(&mut buf_writer, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    
    if color_space == crate::color::TargetColorSpace::Srgb {
        encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    }
    
    let mut writer = encoder.write_header()?;
    
    if color_space == crate::color::TargetColorSpace::DisplayP3 {
        let icc_data = include_bytes!("../assets/profiles/DisplayP3.icc");
        let mut zlib_encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut zlib_encoder, icc_data)?;
        let compressed_icc = zlib_encoder.finish()?;
        
        let mut iccp_payload = Vec::new();
        iccp_payload.extend_from_slice(b"Display P3");
        iccp_payload.push(0); 
        iccp_payload.push(0); 
        iccp_payload.extend_from_slice(&compressed_icc);
        
        writer.write_chunk(png::chunk::ChunkType(*b"iCCP"), &iccp_payload)?;
    }
    
    // ネイティブな eXIf チャンクを注入
    writer.write_chunk(png::chunk::ChunkType(*b"eXIf"), &encoded_metadata)?;
    
    // Adobe製品対応のための XMP (iTXt) チャンク構築
    let mut xmp = String::new();
    xmp.push_str("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n");
    xmp.push_str("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"RAWdevRust\">\n");
    xmp.push_str(" <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n");
    xmp.push_str("  <rdf:Description rdf:about=\"\"\n");
    xmp.push_str("    xmlns:exif=\"http://ns.adobe.com/exif/1.0/\"\n");
    xmp.push_str("    xmlns:tiff=\"http://ns.adobe.com/tiff/1.0/\"\n");
    xmp.push_str("    xmlns:aux=\"http://ns.adobe.com/exif/1.0/aux/\">\n");
    
    if let Some(make) = &exif_info.make {
        xmp.push_str(&format!("   <tiff:Make>{}</tiff:Make>\n", html_escape(make)));
    }
    if let Some(model) = &exif_info.model {
        xmp.push_str(&format!("   <tiff:Model>{}</tiff:Model>\n", html_escape(model)));
    }
    if let Some(lens) = &exif_info.lens_model {
        xmp.push_str(&format!("   <aux:Lens>{}</aux:Lens>\n", html_escape(lens)));
        xmp.push_str(&format!("   <aux:LensModel>{}</aux:LensModel>\n", html_escape(lens)));
    }
    if let Some(dt) = &exif_info.datetime {
        let dt_iso = if dt.len() >= 19 {
            format!("{}-{}-{}T{}", &dt[0..4], &dt[5..7], &dt[8..10], &dt[11..])
        } else {
            dt.clone()
        };
        xmp.push_str(&format!("   <exif:DateTimeOriginal>{}</exif:DateTimeOriginal>\n", html_escape(&dt_iso)));
    }
    if let Some((n, d)) = exif_info.f_number {
        xmp.push_str(&format!("   <exif:FNumber>{}/{}</exif:FNumber>\n", n, d));
    }
    if let Some((n, d)) = exif_info.exposure_time {
        xmp.push_str(&format!("   <exif:ExposureTime>{}/{}</exif:ExposureTime>\n", n, d));
    }
    if let Some((n, d)) = exif_info.focal_length {
        xmp.push_str(&format!("   <exif:FocalLength>{}/{}</exif:FocalLength>\n", n, d));
    }
    if let Some(iso) = exif_info.iso {
        xmp.push_str("   <exif:ISOSpeedRatings>\n    <rdf:Seq>\n");
        xmp.push_str(&format!("     <rdf:li>{}</rdf:li>\n", iso));
        xmp.push_str("    </rdf:Seq>\n   </exif:ISOSpeedRatings>\n");
    }
    if let Some(mm) = exif_info.metering_mode {
        xmp.push_str(&format!("   <exif:MeteringMode>{}</exif:MeteringMode>\n", mm));
    }
    if let Some(ep) = exif_info.exposure_program {
        xmp.push_str(&format!("   <exif:ExposureProgram>{}</exif:ExposureProgram>\n", ep));
    }
    if let Some(wb) = exif_info.white_balance {
        xmp.push_str(&format!("   <exif:WhiteBalance>{}</exif:WhiteBalance>\n", wb));
    }
    if let Some((n, d)) = exif_info.exposure_bias {
        xmp.push_str(&format!("   <exif:ExposureBiasValue>{}/{}</exif:ExposureBiasValue>\n", n, d));
    }
    xmp.push_str("  </rdf:Description>\n </rdf:RDF>\n</x:xmpmeta>\n");
    xmp.push_str("<?xpacket end=\"w\"?>\n");

    let mut itxt_data = Vec::new();
    itxt_data.extend_from_slice(b"XML:com.adobe.xmp\0"); // Keyword
    itxt_data.push(0); // Compression flag (0: uncompressed)
    itxt_data.push(0); // Compression method (0: zlib)
    itxt_data.push(0); // Language tag (empty string)
    itxt_data.push(0); // Translated keyword (empty string)
    itxt_data.extend_from_slice(xmp.as_bytes()); // Text

    writer.write_chunk(png::chunk::ChunkType(*b"iTXt"), &itxt_data)?;
    
    writer.write_image_data(rgb)?;

    Ok(())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&apos;")
}
