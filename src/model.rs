use crate::three;
use crate::screen::Rgb;
use std::*;
use std::process::Command;

#[derive(Debug)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl error::Error for ParseError {}

#[derive(Clone)]
pub struct ColoredEdge {
    pub start: three::Point,
    pub end: three::Point,
    pub start_color: Rgb,
    pub end_color: Rgb,
    pub start_t: f32,
    pub end_t: f32,
}

pub struct Model {
    pub points: Vec<three::Point>,
    pub edges: Vec<(three::Point, three::Point)>,
    pub colored_edges: Vec<ColoredEdge>,
    pub position: three::Point,
}

impl Model {
    pub fn model_to_world(&self, point: &three::Point) -> three::Point {
        three::Point {
            x: point.x + self.position.x,
            y: point.y + self.position.y,
            z: point.z + self.position.z,
        }
    }

    pub fn world_bounds(&self) -> (three::Point, three::Point) {
        if self.points.is_empty() && self.edges.is_empty() && self.colored_edges.is_empty() {
            return (
                three::Point::new(0., 0., 0.),
                three::Point::new(0., 0., 0.),
            );
        }

        let first_point = if !self.points.is_empty() {
            self.points[0]
        } else if !self.edges.is_empty() {
            self.edges[0].0
        } else {
            self.colored_edges[0].start
        };

        let mut min_bounds = first_point;
        let mut max_bounds = first_point;

        for point in &self.points {
            min_bounds.x = f32::min(point.x, min_bounds.x);
            min_bounds.y = f32::min(point.y, min_bounds.y);
            min_bounds.z = f32::min(point.z, min_bounds.z);
            max_bounds.x = f32::max(point.x, max_bounds.x);
            max_bounds.y = f32::max(point.y, max_bounds.y);
            max_bounds.z = f32::max(point.z, max_bounds.z);
        }

        for (start, end) in &self.edges {
            for point in [start, end] {
                min_bounds.x = f32::min(point.x, min_bounds.x);
                min_bounds.y = f32::min(point.y, min_bounds.y);
                min_bounds.z = f32::min(point.z, min_bounds.z);
                max_bounds.x = f32::max(point.x, max_bounds.x);
                max_bounds.y = f32::max(point.y, max_bounds.y);
                max_bounds.z = f32::max(point.z, max_bounds.z);
            }
        }

        for edge in &self.colored_edges {
            for point in [&edge.start, &edge.end] {
                min_bounds.x = f32::min(point.x, min_bounds.x);
                min_bounds.y = f32::min(point.y, min_bounds.y);
                min_bounds.z = f32::min(point.z, min_bounds.z);
                max_bounds.x = f32::max(point.x, max_bounds.x);
                max_bounds.y = f32::max(point.y, max_bounds.y);
                max_bounds.z = f32::max(point.z, max_bounds.z);
            }
        }

        (min_bounds, max_bounds)
    }

    pub fn apply_color_scheme<F>(&mut self, color_fn: F)
    where
        F: Fn(f32) -> Rgb,
    {
        for edge in &mut self.colored_edges {
            edge.start_color = color_fn(edge.start_t);
            edge.end_color = color_fn(edge.end_t);
        }
    }
}

fn load_obj_colored(path: &str, position: three::Point) -> Result<Model, Box<dyn error::Error>> {
    let mut code = fs::read_to_string(path)?;
    code = code.replace("\\\n", " ");

    let mut vertices = Vec::<three::Point>::new();
    let mut faces = Vec::<Vec<usize>>::new();

    for line in code.split('\n') {
        let mut tokens = line.split_whitespace().filter(|&s| !s.is_empty());

        match tokens.next() {
            Some("v") => {
                let coords: Vec<&str> = tokens.collect();
                if coords.len() >= 3 {
                    let x = coords[0].parse::<f32>()?;
                    let y = coords[1].parse::<f32>()?;
                    let z = coords[2].parse::<f32>()?;
                    vertices.push(three::Point::new(x, y, z));
                }
            }
            Some("f") | Some("fo") => {
                let mut face = Vec::<usize>::new();
                for point in tokens {
                    if let Some(vertex_str) = point.split('/').next() {
                        if let Ok(vertex_index) = vertex_str.parse::<usize>() {
                            if let Some(idx) = vertex_index.checked_sub(1) {
                                face.push(idx);
                            }
                        }
                    }
                }
                if face.len() >= 2 {
                    faces.push(face);
                }
            }
            _ => {}
        }
    }

    if vertices.is_empty() {
        return Err(Box::new(ParseError("No vertices found in OBJ".to_string())));
    }

    let mut min_idx = usize::MAX;
    let mut max_idx = 0usize;
    for face in &faces {
        for &idx in face {
            min_idx = min_idx.min(idx);
            max_idx = max_idx.max(idx);
        }
    }
    let idx_range = if max_idx > min_idx { max_idx - min_idx } else { 1 };
    let mut colored_edges: Vec<ColoredEdge> = Vec::new();

    for face in &faces {
        if face.len() >= 2 {
            for i in 0..face.len() {
                let start_idx = face[i];
                let end_idx = face[(i + 1) % face.len()];

                if start_idx < vertices.len() && end_idx < vertices.len() {
                    let t1 = (start_idx - min_idx) as f32 / idx_range as f32;
                    let t2 = (end_idx - min_idx) as f32 / idx_range as f32;

                    colored_edges.push(ColoredEdge {
                        start: vertices[start_idx],
                        end: vertices[end_idx],
                        start_color: Rgb::white(),
                        end_color: Rgb::white(),
                        start_t: t1,
                        end_t: t2,
                    });
                }
            }
        }
    }

    colored_edges.sort_by(|a, b| {
        let a_key = (
            (a.start.x * 1000.0) as i32,
            (a.start.y * 1000.0) as i32,
            (a.start.z * 1000.0) as i32,
            (a.end.x * 1000.0) as i32,
            (a.end.y * 1000.0) as i32,
            (a.end.z * 1000.0) as i32,
        );
        let b_key = (
            (b.start.x * 1000.0) as i32,
            (b.start.y * 1000.0) as i32,
            (b.start.z * 1000.0) as i32,
            (b.end.x * 1000.0) as i32,
            (b.end.y * 1000.0) as i32,
            (b.end.z * 1000.0) as i32,
        );
        a_key.cmp(&b_key)
    });
    colored_edges.dedup_by(|a, b| {
        (a.start.x - b.start.x).abs() < 0.001
            && (a.start.y - b.start.y).abs() < 0.001
            && (a.start.z - b.start.z).abs() < 0.001
            && (a.end.x - b.end.x).abs() < 0.001
            && (a.end.y - b.end.y).abs() < 0.001
            && (a.end.z - b.end.z).abs() < 0.001
    });

    const MIN_EDGE_LENGTH: f32 = 0.1;
    colored_edges.retain(|e| {
        let dx = e.end.x - e.start.x;
        let dy = e.end.y - e.start.y;
        let dz = e.end.z - e.start.z;
        dx * dx + dy * dy + dz * dz >= MIN_EDGE_LENGTH * MIN_EDGE_LENGTH
    });

    const MAX_EDGES: usize = 50000;
    if colored_edges.len() > MAX_EDGES {
        let step = (colored_edges.len() as f32 / MAX_EDGES as f32).ceil() as usize;
        colored_edges = colored_edges.into_iter()
            .enumerate()
            .filter(|(i, _)| i % step == 0)
            .map(|(_, e)| e)
            .collect();
    }

    Ok(Model {
        points: vertices,
        edges: Vec::new(),
        colored_edges,
        position,
    })
}

fn get_cache_dir() -> Result<path::PathBuf, Box<dyn error::Error>> {
    let home = env::var("HOME").map_err(|_| ParseError("HOME not set".to_string()))?;
    let cache_dir = path::PathBuf::from(home).join(".cache").join("pepterm");
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)?;
    }
    Ok(cache_dir)
}

pub fn cache_info() -> Result<(usize, u64, path::PathBuf), Box<dyn error::Error>> {
    let cache_dir = get_cache_dir()?;
    let mut count = 0;
    let mut total_size = 0u64;

    if cache_dir.exists() {
        for entry in fs::read_dir(&cache_dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                count += 1;
                total_size += metadata.len();
            }
        }
    }

    Ok((count, total_size, cache_dir))
}

pub fn cache_clear() -> Result<usize, Box<dyn error::Error>> {
    let cache_dir = get_cache_dir()?;
    let mut count = 0;

    if cache_dir.exists() {
        for entry in fs::read_dir(&cache_dir)? {
            let entry = entry?;
            if entry.metadata()?.is_file() {
                fs::remove_file(entry.path())?;
                count += 1;
            }
        }
    }

    Ok(count)
}

fn check_pymol() -> Result<(), Box<dyn error::Error>> {
    let pymol_check = Command::new("which").arg("pymol").output();
    if pymol_check.is_err() || !pymol_check.unwrap().status.success() {
        return Err(Box::new(ParseError(
            "PyMOL not found. Install with: brew install pymol".to_string(),
        )));
    }
    Ok(())
}

fn export_cartoon_with_pymol(pdb_input: &str, chain: Option<&str>) -> Result<String, Box<dyn error::Error>> {
    check_pymol()?;

    let cache_dir = get_cache_dir()?;
    let pdb_id = pdb_input.to_uppercase();
    let obj_filename = match chain {
        Some(c) => format!("{}_{}.obj", pdb_id, c.to_uppercase()),
        None => format!("{}.obj", pdb_id),
    };
    let obj_path = cache_dir.join(&obj_filename);

    if obj_path.exists() {
        eprintln!("Using cached structure from {:?}", obj_path);
        return Ok(obj_path.to_string_lossy().to_string());
    }

    let selection_cmd = match chain {
        Some(c) => format!("select sel, chain {}\nhide everything\nshow cartoon, sel", c.to_uppercase()),
        None => "hide everything\nshow cartoon".to_string(),
    };

    let pymol_script = format!(
        r#"
set fetch_path, {}
fetch {}, async=0
{}
set cartoon_sampling, 3
save {}
quit
"#,
        cache_dir.display(), pdb_id, selection_cmd, obj_path.display()
    );

    let script_path = cache_dir.join("pymol_script.pml");
    fs::write(&script_path, &pymol_script)?;

    eprintln!("Fetching {} and generating cartoon with PyMOL...", pdb_id);

    let output = Command::new("pymol")
        .args(["-cq", &script_path.to_string_lossy()])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Box::new(ParseError(format!("PyMOL failed: {}", stderr))));
    }

    if !obj_path.exists() {
        return Err(Box::new(ParseError(
            "PyMOL did not create OBJ file. Check PDB ID.".to_string(),
        )));
    }

    eprintln!("Cached to {:?}", obj_path);
    Ok(obj_path.to_string_lossy().to_string())
}

fn export_cartoon_from_file(file_path: &str, chain: Option<&str>) -> Result<String, Box<dyn error::Error>> {
    check_pymol()?;

    let cache_dir = get_cache_dir()?;
    let abs_path = fs::canonicalize(file_path)?;
    let file_stem = abs_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let obj_filename = match chain {
        Some(c) => format!("local_{}_{}.obj", file_stem, c.to_uppercase()),
        None => format!("local_{}.obj", file_stem),
    };
    let obj_path = cache_dir.join(&obj_filename);

    let selection_cmd = match chain {
        Some(c) => format!("select sel, chain {}\nhide everything\nshow cartoon, sel", c.to_uppercase()),
        None => "hide everything\nshow cartoon".to_string(),
    };

    let pymol_script = format!(
        r#"
load {}
{}
set cartoon_sampling, 3
save {}
quit
"#,
        abs_path.display(),
        selection_cmd,
        obj_path.display()
    );

    let script_path = cache_dir.join("pymol_script.pml");
    fs::write(&script_path, &pymol_script)?;

    eprintln!("Generating cartoon with PyMOL...");

    let output = Command::new("pymol")
        .args(["-cq", &script_path.to_string_lossy()])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Box::new(ParseError(format!("PyMOL failed: {}", stderr))));
    }

    if !obj_path.exists() {
        return Err(Box::new(ParseError(
            "PyMOL did not create OBJ file.".to_string(),
        )));
    }

    Ok(obj_path.to_string_lossy().to_string())
}

pub fn new_cartoon(input: &str, chain: Option<&str>, position: three::Point) -> Result<Model, Box<dyn error::Error>> {
    if input.ends_with(".obj") {
        return load_obj_colored(input, position);
    }

    if input.ends_with(".pdb") || input.ends_with(".cif") || input.contains('/') || input.contains('\\') {
        let obj_path = export_cartoon_from_file(input, chain)?;
        return load_obj_colored(&obj_path, position);
    }

    let obj_path = export_cartoon_with_pymol(input, chain)?;
    load_obj_colored(&obj_path, position)
}

pub fn search_pdb(query: &str) -> Result<Vec<PdbSearchResult>, Box<dyn error::Error>> {
    let search_url = "https://search.rcsb.org/rcsbsearch/v2/query";

    let search_json = format!(r#"{{
        "query": {{
            "type": "terminal",
            "service": "full_text",
            "parameters": {{
                "value": "{}"
            }}
        }},
        "return_type": "entry",
        "request_options": {{
            "paginate": {{
                "start": 0,
                "rows": 10
            }},
            "results_content_type": ["experimental"]
        }}
    }}"#, query);

    let output = Command::new("curl")
        .args([
            "-s",
            "-X", "POST",
            "-H", "Content-Type: application/json",
            "-d", &search_json,
            search_url
        ])
        .output()?;

    if !output.status.success() {
        return Err(Box::new(ParseError("Search request failed".to_string())));
    }

    let response = String::from_utf8_lossy(&output.stdout);
    parse_search_results(&response)
}

#[derive(Debug, Clone)]
pub struct PdbSearchResult {
    pub pdb_id: String,
    pub title: String,
}

fn parse_search_results(json: &str) -> Result<Vec<PdbSearchResult>, Box<dyn error::Error>> {
    let mut results = Vec::new();
    let mut pos = 0;
    while let Some(id_start) = json[pos..].find("\"identifier\"") {
        let abs_pos = pos + id_start;
        let rest = &json[abs_pos..];
        if let Some(colon_pos) = rest.find(':') {
            let after_colon = &rest[colon_pos + 1..];
            if let Some(quote_start) = after_colon.find('"') {
                let value_start = &after_colon[quote_start + 1..];
                if let Some(quote_end) = value_start.find('"') {
                    let pdb_id = value_start[..quote_end].to_string();
                    if pdb_id.len() == 4 && pdb_id.chars().all(|c| c.is_alphanumeric()) {
                        results.push(PdbSearchResult {
                            pdb_id,
                            title: String::new(),
                        });
                    }
                    pos = abs_pos + id_start + colon_pos + quote_start + quote_end + 2;
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    for result in &mut results {
        if let Ok(title) = fetch_pdb_title(&result.pdb_id) {
            result.title = title;
        }
    }

    Ok(results)
}

fn fetch_pdb_title(pdb_id: &str) -> Result<String, Box<dyn error::Error>> {
    let url = format!("https://data.rcsb.org/rest/v1/core/entry/{}", pdb_id);

    let output = Command::new("curl")
        .args(["-s", &url])
        .output()?;

    let response = String::from_utf8_lossy(&output.stdout);

    if let Some(title_start) = response.find("\"title\"") {
        let rest = &response[title_start + 9..];
        if let Some(quote_end) = rest.find('"') {
            return Ok(rest[..quote_end].to_string());
        }
    }

    Ok(String::new())
}

#[allow(dead_code)]
pub fn get_pdb_chains(pdb_id: &str) -> Result<Vec<String>, Box<dyn error::Error>> {
    let url = format!("https://data.rcsb.org/rest/v1/core/entry/{}", pdb_id);

    let output = Command::new("curl")
        .args(["-s", &url])
        .output()?;

    let response = String::from_utf8_lossy(&output.stdout);
    let mut chains = Vec::new();
    let mut pos = 0;
    while let Some(chain_start) = response[pos..].find("\"auth_asym_id\"") {
        let id_pos = pos + chain_start + 16;
        if let Some(quote_end) = response[id_pos..].find('"') {
            let chain = response[id_pos..id_pos + quote_end].to_string();
            if !chains.contains(&chain) {
                chains.push(chain);
            }
            pos = id_pos + quote_end;
        } else {
            break;
        }
    }

    chains.sort();
    Ok(chains)
}
