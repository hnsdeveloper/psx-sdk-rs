use walkdir::WalkDir;
use xml_builder::{XMLBuilder, XMLElement, XMLVersion};

fn traverse_assets(parent: &mut XMLElement, path: &str) -> Result<(), walkdir::Error> {
    let wd = WalkDir::new(path)
        .max_depth(1)
        .follow_links(false)
        .sort_by(|a, b| {
            let is_a_dir = a.metadata().unwrap().is_dir();
            let is_b_dir = b.metadata().unwrap().is_dir();
            if is_a_dir && !is_b_dir {
                return std::cmp::Ordering::Less;
            } else if !is_a_dir && is_b_dir {
                return std::cmp::Ordering::Greater;
            }
            a.file_name().cmp(b.file_name())
        });
    for e in wd {
        let entry = e?;
        if entry.path().to_str().unwrap() == path {
            continue;
        }
        let child = if entry.metadata()?.is_dir() {
            let mut dir = XMLElement::new("dir");
            dir.add_attribute("name", &entry.file_name().to_str().unwrap().to_uppercase());
            traverse_assets(&mut dir, entry.path().as_os_str().to_str().unwrap())?;
            dir
        } else {
            let mut file = XMLElement::new("file");
            file.add_attribute("name", &entry.file_name().to_str().unwrap().to_uppercase());
            file.add_attribute("type", "data");
            file.add_attribute("source", entry.path().to_str().unwrap());
            file
        };
        // Safe to ignore error as we are not adding to a text element.
        _ = parent.add_child(child);
    }
    Ok(())
}

pub fn generate_xml(
    path: &str, image_name: &str, app_id: &str, target_path: &str, volume: Option<&String>,
    publisher: Option<&String>,
) -> Result<Vec<u8>, std::io::Error> {
    let mut xml = XMLBuilder::new()
        .version(XMLVersion::XML1_1)
        .encoding("UTF-8".into())
        .build();

    let mut iso_project = XMLElement::new("iso_project");
    iso_project.add_attribute("image_name", &format!("{}.bin", image_name));
    iso_project.add_attribute("cue_sheet", &format!("{}.cue", image_name));
    iso_project.add_attribute("no_xa", "0");

    let mut track = XMLElement::new("track");
    track.add_attribute("type", "data");

    let mut identifiers = XMLElement::new("identifiers");
    identifiers.add_attribute("system", "PLAYSTATION");
    identifiers.add_attribute("application", "PLAYSTATION");
    if let Some(volume_name) = volume {
        identifiers.add_attribute("volume", &volume_name);
    }
    if let Some(publisher_name) = publisher {
        identifiers.add_attribute("publisher", &publisher_name);
    }

    let mut directory_tree = XMLElement::new("directory_tree");

    let mut system_cnf = XMLElement::new("file");
    system_cnf.add_attribute("name", "SYSTEM.CNF");
    system_cnf.add_attribute("type", "data");
    system_cnf.add_attribute("source", "SYSTEM.CNF");
    _ = directory_tree.add_child(system_cnf);

    let mut exec_file = XMLElement::new("file");
    exec_file.add_attribute("name", &format!("{}.EXE", app_id.to_ascii_uppercase()));
    exec_file.add_attribute("source", target_path);
    exec_file.add_attribute("type", "data");
    _ = directory_tree.add_child(exec_file);

    match traverse_assets(&mut directory_tree, path) {
        Ok(_) => {},
        Err(err) => return Err(err.into_io_error().unwrap()),
    };

    let mut dummy = XMLElement::new("dummy");
    dummy.add_attribute("sectors", "1024");
    _ = directory_tree.add_child(dummy);

    // Safe to ignore all the errors, as the error that could be raised is if adding
    // children to a text element.
    _ = track.add_child(identifiers);
    _ = track.add_child(directory_tree);
    _ = iso_project.add_child(track);

    xml.set_root_element(iso_project);
    let mut v = Vec::new();
    xml.generate(&mut v).unwrap();

    Ok(v)
}
