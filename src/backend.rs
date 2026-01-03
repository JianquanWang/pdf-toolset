use std::path::Path;
use std::path::PathBuf;
use lopdf::{Document, Object};
use std::collections::{HashMap, HashSet};


pub fn merge(inputs: &[PathBuf], output: &Path) -> Result<(), String> {
    use lopdf::{Object, ObjectId, Bookmark};
    use std::collections::BTreeMap;
    
    if inputs.is_empty() {
        return Err("No input files provided".into());
    }

    let mut documents: Vec<Document> = Vec::new();
    for p in inputs {
        let d = Document::load(p).map_err(|e| format!("Failed to load {}: {}", p.display(), e))?;
        documents.push(d);
    }

    let mut max_id: u32 = 1;
    let mut pagenum: usize = 1;
    let mut documents_pages: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut documents_objects: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut document = Document::with_version("1.5");

    for mut doc in documents {
        let mut first = false;
        doc.renumber_objects_with(max_id);

        max_id = doc.max_id + 1;

        documents_pages.extend(
            doc.get_pages()
                .into_iter()
                .map(|(_, object_id)| {
                    if !first {
                        let bookmark = Bookmark::new(String::from(format!("Page_{}", pagenum)), [0.0, 0.0, 1.0], 0, object_id);
                        document.add_bookmark(bookmark, None);
                        first = true;
                        pagenum += 1;
                    }

                    (
                        object_id,
                        doc.get_object(object_id).unwrap().to_owned(),
                    )
                })
                .collect::<BTreeMap<ObjectId, Object>>(),
        );

        documents_objects.extend(doc.objects);
    }

    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    // Process all objects except Page type
    for (object_id, object) in documents_objects.iter() {
        match object.type_name().unwrap_or("") {
            "Catalog" => {
                catalog_object = Some((if let Some((id, _)) = catalog_object { id } else { *object_id }, object.clone()));
            }
            "Pages" => {
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref object)) = pages_object {
                        if let Ok(old_dictionary) = object.as_dict() {
                            dictionary.extend(&old_dictionary.clone());
                        }
                    }

                    pages_object = Some((if let Some((id, _)) = pages_object { id } else { *object_id }, Object::Dictionary(dictionary)));
                }
            }
            "Page" => {}
            "Outlines" => {}
            "Outline" => {}
            _ => {
                document.objects.insert(*object_id, object.clone());
            }
        }
    }

    if pages_object.is_none() {
        return Err("Pages root not found in input documents".into());
    }

    for (object_id, object) in documents_pages.into_iter() {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            dictionary.set("Parent", pages_object.as_ref().unwrap().0);
            document.objects.insert(object_id, Object::Dictionary(dictionary));
        }
    }

    if catalog_object.is_none() {
        return Err("Catalog root not found in input documents".into());
    }

    let catalog_object = catalog_object.unwrap();
    let pages_object = pages_object.unwrap();

    if let Ok(dictionary) = pages_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        let count = document.objects.iter().filter(|(_, obj)| obj.type_name().unwrap_or("") == "Page").count();
        dictionary.set("Count", count as u32);
        let kids: Vec<_> = document.objects.iter().filter_map(|(id, obj)| {
            if obj.type_name().unwrap_or("") == "Page" {
                Some(Object::Reference(*id))
            } else {
                None
            }
        }).collect();
        dictionary.set("Kids", kids);
        document.objects.insert(pages_object.0, Object::Dictionary(dictionary));
    }

    if let Ok(dictionary) = catalog_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", pages_object.0);
        dictionary.remove(b"Outlines");
        document.objects.insert(catalog_object.0, Object::Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_object.0);

    document.max_id = document.objects.len() as u32;
    document.renumber_objects();
    document.adjust_zero_pages();
    if let Some(n) = document.build_outline() {
        if let Ok(x) = document.get_object_mut(catalog_object.0) {
            if let Object::Dictionary(ref mut dict) = x {
                dict.set("Outlines", Object::Reference(n));
            }
        }
    }

    document.compress();
    document.save(output).map_err(|e| format!("Failed to save merged PDF: {}", e))?;
    Ok(())
}

pub fn split(_input: &Path, _output_dir: &Path) -> Result<(), String> {
    use lopdf::{Dictionary, Stream};

    let src = Document::load(_input).map_err(|e| format!("Failed to load {}: {}", _input.display(), e))?;

    fn collect_refs(obj: &lopdf::Object, doc: &Document, set: &mut HashSet<lopdf::ObjectId>) {
        match obj {
            lopdf::Object::Reference(id) => {
                if set.insert(*id) {
                    if let Some(o) = doc.objects.get(id) {
                        collect_refs(o, doc, set);
                    }
                }
            }
            lopdf::Object::Array(arr) => {
                for item in arr {
                    collect_refs(item, doc, set);
                }
            }
            lopdf::Object::Dictionary(d) => {
                for (_k, v) in d.iter() {
                    collect_refs(v, doc, set);
                }
            }
            lopdf::Object::Stream(s) => {
                for (_k, v) in s.dict.iter() {
                    collect_refs(v, doc, set);
                }
            }
            _ => {}
        }
    }

    let pages = src.get_pages();
    let final_out_dir = if let Some(stem) = _input.file_stem().and_then(|s| s.to_str()) {
        if let Some(fname) = _output_dir.file_name().and_then(|s| s.to_str()) {
            if fname == format!("{}-pages", stem) {
                _output_dir.to_path_buf()
            } else {
                let p = _output_dir.join(format!("{}-pages", stem));
                let _ = std::fs::create_dir_all(&p);
                p
            }
        } else {
            let p = _output_dir.join(format!("{}-pages", stem));
            let _ = std::fs::create_dir_all(&p);
            p
        }
    } else {
        let _ = std::fs::create_dir_all(_output_dir);
        _output_dir.to_path_buf()
    };

    for (num, page_id) in pages.into_iter() {
        let mut refs: HashSet<lopdf::ObjectId> = HashSet::new();
        refs.insert(page_id);
        if let Ok(page_obj) = src.get_object(page_id) {
            collect_refs(page_obj, &src, &mut refs);
        }

        let mut mapping: HashMap<lopdf::ObjectId, lopdf::ObjectId> = HashMap::new();
        let mut out_doc = Document::with_version(&src.version);
        let pages_id = out_doc.new_object_id();

        for old_id in refs.iter() {
            let new_id = out_doc.new_object_id();
            mapping.insert(*old_id, new_id);
        }

        fn remap_object(obj: &lopdf::Object, mapping: &HashMap<lopdf::ObjectId, lopdf::ObjectId>) -> lopdf::Object {
            match obj {
                lopdf::Object::Reference(r) => {
                    if let Some(n) = mapping.get(r) {
                        lopdf::Object::Reference(*n)
                    } else {
                        lopdf::Object::Reference(*r)
                    }
                }
                lopdf::Object::Array(arr) => {
                    lopdf::Object::Array(arr.iter().map(|o| remap_object(o, mapping)).collect())
                }
                lopdf::Object::Dictionary(d) => {
                    let mut nd = Dictionary::new();
                    for (k, v) in d.iter() {
                        nd.set(k.clone(), remap_object(v, mapping));
                    }
                    lopdf::Object::Dictionary(nd)
                }
                lopdf::Object::Stream(s) => {
                    let mut nd = Dictionary::new();
                    for (k, v) in s.dict.iter() {
                        nd.set(k.clone(), remap_object(v, mapping));
                    }
                    lopdf::Object::Stream(Stream::new(nd, s.content.clone()))
                }
                other => other.clone(),
            }
        }

        for (old_id, new_id) in mapping.iter() {
            if let Some(old_obj) = src.objects.get(old_id) {
                let mut new_obj = remap_object(old_obj, &mapping);
                if *old_id == page_id {
                    if let lopdf::Object::Dictionary(ref mut dict) = new_obj {
                        dict.set("Parent", pages_id);
                    }
                }
                out_doc.objects.insert(*new_id, new_obj);
            }
        }

        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
        pages_dict.set("Kids", lopdf::Object::Array(vec![lopdf::Object::Reference(mapping[&page_id])]));
        pages_dict.set("Count", lopdf::Object::Integer(1));
        out_doc.objects.insert(pages_id, lopdf::Object::Dictionary(pages_dict));

        let catalog_id = out_doc.new_object_id();
        let mut catalog_dict = Dictionary::new();
        catalog_dict.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
        catalog_dict.set("Pages", lopdf::Object::Reference(pages_id));
        out_doc.objects.insert(catalog_id, lopdf::Object::Dictionary(catalog_dict));
        out_doc.trailer.set("Root", catalog_id);

        out_doc.max_id = out_doc.objects.len() as u32;
        out_doc.renumber_objects();
        out_doc.compress();

        let out_path = final_out_dir.join(format!("page-{}.pdf", num));
        out_doc.save(&out_path).map_err(|e| format!("Failed to save split page {}: {}", num, e))?;
    }

    Ok(())
}

pub fn extract_text(_input: &Path, _output_txt: &Path) -> Result<(), String> {
    let doc = Document::load(_input).map_err(|e| format!("Failed to load {}: {}", _input.display(), e))?;
    let pages_map = doc.get_pages();
    let mut page_numbers: Vec<u32> = pages_map.keys().cloned().collect();
    page_numbers.sort();
    let text = doc.extract_text(&page_numbers).map_err(|e| format!("Failed to extract text: {}", e))?;
    std::fs::write(_output_txt, text).map_err(|e| format!("Failed to write text file: {}", e))?;
    Ok(())
}

pub fn compress(_input: &Path, _output: &Path) -> Result<(), String> {
    use image::imageops::FilterType;
    use image::{GenericImageView, DynamicImage};

    let mut doc = Document::load(_input).map_err(|e| format!("Failed to load {}: {}", _input.display(), e))?;

    let (quality, scale) = (75u8, 0.75f32);

    let object_ids: Vec<_> = doc.objects.keys().cloned().collect();
    for id in object_ids {
        if let Some(obj) = doc.objects.get_mut(&id) {
            if let Object::Stream(ref mut stream) = obj {
                let dict = &stream.dict;
                let is_xobj = match dict.get(b"Type") {
                    Ok(o) => o.as_name().map(|n| n == b"XObject").unwrap_or(false),
                    Err(_) => false,
                };
                let is_image = match dict.get(b"Subtype") {
                    Ok(o) => o.as_name().map(|n| n == b"Image").unwrap_or(false),
                    Err(_) => false,
                };

                if is_xobj && is_image {
                    if let Ok(img) = image::load_from_memory(&stream.content) {
                        let (w, h) = img.dimensions();
                        let new_w = ((w as f32) * scale).max(1.0) as u32;
                        let new_h = ((h as f32) * scale).max(1.0) as u32;
                        let img2: DynamicImage = if (new_w, new_h) != (w, h) {
                            let resized = image::imageops::resize(&img.to_rgba8(), new_w, new_h, FilterType::Lanczos3);
                            DynamicImage::ImageRgba8(resized)
                        } else {
                            img.clone()
                        };
                        let mut out = Vec::new();
                        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
                        if encoder.encode_image(&img2).is_err() {
                            continue;
                        }
                        stream.content = out;
                        stream.dict.set("Filter", lopdf::Object::Name(b"DCTDecode".to_vec()));
                        stream.dict.set("ColorSpace", lopdf::Object::Name(b"DeviceRGB".to_vec()));
                        stream.dict.set("BitsPerComponent", lopdf::Object::Integer(8));
                        stream.allows_compression = true;
                    }
                }
            }
        }
    }

    doc.compress();
    doc.save(_output).map_err(|e| format!("Failed to save compressed PDF: {}", e))?;
    Ok(())
}

pub fn rotate(_input: &Path, _output: &Path, _degrees: i32, pages: Option<Vec<u32>>) -> Result<(), String> {
    let mut doc = Document::load(_input).map_err(|e| format!("Failed to load {}: {}", _input.display(), e))?;

    let mut deg = ((_degrees % 360) + 360) % 360;
    if deg % 90 != 0 {
        return Err("Rotation must be a multiple of 90 degrees".into());
    }

    if deg == 0 {
        // just copy
        return std::fs::copy(_input, _output)
            .map(|_| ())
            .map_err(|e| format!("Failed to copy file for zero-rotation: {}", e));
    }

    let pages_set: Option<std::collections::HashSet<u32>> = pages.map(|v| v.into_iter().collect());

    for (page_number, page_id) in doc.get_pages().into_iter() {
        if let Some(ref set) = pages_set {
            if !set.contains(&page_number) {
                continue;
            }
        }
        if let Ok(obj) = doc.get_object_mut(page_id) {
            if let Object::Dictionary(ref mut dict) = obj {
                let current = if let Ok(o) = dict.get(b"Rotate") {
                    match o.as_i64() {
                        Ok(v) => v,
                        Err(_) => 0,
                    }
                } else { 0 };
                let new = ((current + deg as i64) % 360 + 360) % 360;
                dict.set("Rotate", lopdf::Object::Integer(new));
            }
        }
    }

    doc.save(_output).map_err(|e| format!("Failed to save rotated PDF: {}", e))?;
    Ok(())
}

pub fn convert_to_images(_input: &Path, _output_dir: &Path, _format: &str) -> Result<(), String> {
    if !_input.exists() {
        return Err(format!("Input PDF not found: {}", _input.display()));
    }
    let format_flag = match _format.to_lowercase().as_str() {
        "png" => "-png",
        "jpeg" | "jpg" => "-jpeg",
        other => return Err(format!("Unsupported image format: {}", other)),
    };

    let final_out_dir = if let Some(stem) = _input.file_stem().and_then(|s| s.to_str()) {
        if let Some(fname) = _output_dir.file_name().and_then(|s| s.to_str()) {
            if fname == format!("{}-images", stem) {
                _output_dir.to_path_buf()
            } else {
                let p = _output_dir.join(format!("{}-images", stem));
                std::fs::create_dir_all(&p).map_err(|e| format!("Failed to create output dir: {}", e))?;
                p
            }
        } else {
            let p = _output_dir.join(format!("{}-images", stem));
            std::fs::create_dir_all(&p).map_err(|e| format!("Failed to create output dir: {}", e))?;
            p
        }
    } else {
        std::fs::create_dir_all(_output_dir).map_err(|e| format!("Failed to create output dir: {}", e))?;
        _output_dir.to_path_buf()
    };

    let prefix = final_out_dir.join(_input.file_stem().and_then(|s| s.to_str()).unwrap_or("page"));
    let prefix_str = prefix.to_string_lossy().to_string();

    let status = std::process::Command::new("pdftoppm")
        .arg(format_flag)
        .arg(_input.to_string_lossy().as_ref())
        .arg(&prefix_str)
        .status()
        .map_err(|e| format!("Failed to spawn pdftoppm: {}", e))?;

    if !status.success() {
        return Err(format!("pdftoppm failed with status: {}", status));
    }

    Ok(())
}
