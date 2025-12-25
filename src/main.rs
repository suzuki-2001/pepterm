// pepterm - View protein structures in your terminal
// Based on terminal3d by Liam Ilan (https://github.com/liam-ilan/terminal3d)

use std::*;
use std::io::Write;
use process::exit;
use time::Duration;

use crossterm::{
    event,
    execute,
    terminal,
    cursor
};

mod screen;
mod three;
mod model;

const VIEWPORT_FOV: f32 = 1.7;
const VIEWPORT_DISTANCE: f32 = 0.1;
const TARGET_DURATION_PER_FRAME: Duration = Duration::from_millis(1000 / 30); // 30 FPS target
const MOUSE_SPEED_MULTIPLIER: f32 = 30.;
const INITIAL_DISTANCE_MULTIPLIER: f32 = 1.2;
const SCROLL_MULTIPLER: f32 = 0.03;
const PAN_MULTIPLIER: f32 = 0.1;
const AUTO_ROTATE_SPEED: f32 = 0.002; // radians per frame (slower rotation)

const HELP_MSG: &str = "\
\x1b[1mpepterm\x1b[0m: View protein structures in your terminal!

\x1b[1mUsage\x1b[0m:
    pepterm <PDB_ID>                   Fetch and view protein from RCSB PDB
    pepterm <PDB_ID> <PDB_ID> ...      View multiple structures side-by-side
    pepterm <file.pdb|.cif>            View local PDB/CIF file
    pepterm <file.obj>                 View OBJ file
    pepterm <ID> --chain <CHAIN>       Show specific chain only
    pepterm search <QUERY>             Search RCSB PDB
    pepterm cache                      Show cache info
    pepterm cache clear                Clear cached files

\x1b[1mOptions\x1b[0m:
    --chain, -n <CHAIN>   Show only the specified chain (e.g., A, B)
    --color, -c <SCHEME>  Specify color scheme

\x1b[1mColor Schemes\x1b[0m:
    coolwarm     Blue to red diverging (default)
    rainbow      N-to-C terminal rainbow
    blues        Sequential blue gradient
    greens       Sequential green gradient
    reds         Sequential red gradient
    oranges      Sequential orange gradient
    purples      Sequential purple gradient
    viridis      Perceptually uniform (blue-green-yellow)
    plasma       Purple to yellow
    magma        Black to white via purple
    inferno      Black to yellow via red
    spectral     Spectral rainbow
    white        White monochrome

\x1b[1mExamples\x1b[0m:
    pepterm 1CRN                  View crambin protein
    pepterm 4HHB                  View hemoglobin
    pepterm 4HHB --chain A        View only chain A
    pepterm 1CRN --color blues    Use blues colormap
    pepterm ./protein.pdb         View local PDB file
    pepterm ./structure.cif       View local CIF file
    pepterm search insulin        Search for insulin structures

\x1b[1mControls\x1b[0m:
    Mouse drag         Rotate around the model (disables auto-rotate)
    Shift + drag       Pan the view
    Scroll up/down     Zoom in/out
    [r]                Toggle auto-rotation
    [c]                Cycle through color schemes
    [0]                Reset view
    [q] or Ctrl+C      Quit

\x1b[1mRequirements\x1b[0m:
    PyMOL must be installed for cartoon rendering.
    Install via: brew install pymol
";

#[derive(Clone, Copy, PartialEq)]
pub enum ColorScheme {
    Rainbow,
    Blues,
    Greens,
    Reds,
    Oranges,
    Purples,
    Viridis,
    Plasma,
    Magma,
    Inferno,
    Coolwarm,
    Spectral,
    White,
}

impl ColorScheme {
    fn from_str(s: &str) -> Option<ColorScheme> {
        match s.to_lowercase().as_str() {
            "rainbow" => Some(ColorScheme::Rainbow),
            "blues" => Some(ColorScheme::Blues),
            "greens" => Some(ColorScheme::Greens),
            "reds" => Some(ColorScheme::Reds),
            "oranges" => Some(ColorScheme::Oranges),
            "purples" => Some(ColorScheme::Purples),
            "viridis" => Some(ColorScheme::Viridis),
            "plasma" => Some(ColorScheme::Plasma),
            "magma" => Some(ColorScheme::Magma),
            "inferno" => Some(ColorScheme::Inferno),
            "coolwarm" => Some(ColorScheme::Coolwarm),
            "spectral" => Some(ColorScheme::Spectral),
            "white" => Some(ColorScheme::White),
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            ColorScheme::Rainbow => "rainbow",
            ColorScheme::Blues => "blues",
            ColorScheme::Greens => "greens",
            ColorScheme::Reds => "reds",
            ColorScheme::Oranges => "oranges",
            ColorScheme::Purples => "purples",
            ColorScheme::Viridis => "viridis",
            ColorScheme::Plasma => "plasma",
            ColorScheme::Magma => "magma",
            ColorScheme::Inferno => "inferno",
            ColorScheme::Coolwarm => "coolwarm",
            ColorScheme::Spectral => "spectral",
            ColorScheme::White => "white",
        }
    }

    fn next(&self) -> ColorScheme {
        match self {
            ColorScheme::Rainbow => ColorScheme::Blues,
            ColorScheme::Blues => ColorScheme::Greens,
            ColorScheme::Greens => ColorScheme::Reds,
            ColorScheme::Reds => ColorScheme::Oranges,
            ColorScheme::Oranges => ColorScheme::Purples,
            ColorScheme::Purples => ColorScheme::Viridis,
            ColorScheme::Viridis => ColorScheme::Plasma,
            ColorScheme::Plasma => ColorScheme::Magma,
            ColorScheme::Magma => ColorScheme::Inferno,
            ColorScheme::Inferno => ColorScheme::Coolwarm,
            ColorScheme::Coolwarm => ColorScheme::Spectral,
            ColorScheme::Spectral => ColorScheme::White,
            ColorScheme::White => ColorScheme::Rainbow,
        }
    }

    fn get_color(&self, t: f32) -> screen::Rgb {
        let t = t.clamp(0.0, 1.0);
        match self {
            ColorScheme::Rainbow => Self::rainbow(t),
            ColorScheme::Blues => Self::blues(t),
            ColorScheme::Greens => Self::greens(t),
            ColorScheme::Reds => Self::reds(t),
            ColorScheme::Oranges => Self::oranges(t),
            ColorScheme::Purples => Self::purples(t),
            ColorScheme::Viridis => Self::viridis(t),
            ColorScheme::Plasma => Self::plasma(t),
            ColorScheme::Magma => Self::magma(t),
            ColorScheme::Inferno => Self::inferno(t),
            ColorScheme::Coolwarm => Self::coolwarm(t),
            ColorScheme::Spectral => Self::spectral(t),
            ColorScheme::White => screen::Rgb::new(255, 255, 255),
        }
    }

    fn rainbow(t: f32) -> screen::Rgb {
        if t < 0.25 {
            let s = t / 0.25;
            screen::Rgb::new(0, (s * 255.0) as u8, 255)
        } else if t < 0.5 {
            let s = (t - 0.25) / 0.25;
            screen::Rgb::new(0, 255, (255.0 * (1.0 - s)) as u8)
        } else if t < 0.75 {
            let s = (t - 0.5) / 0.25;
            screen::Rgb::new((s * 255.0) as u8, 255, 0)
        } else {
            let s = (t - 0.75) / 0.25;
            screen::Rgb::new(255, (255.0 * (1.0 - s)) as u8, 0)
        }
    }

    fn blues(t: f32) -> screen::Rgb {
        let colors = [
            (247, 251, 255), (222, 235, 247), (198, 219, 239), (158, 202, 225),
            (107, 174, 214), (66, 146, 198), (33, 113, 181), (8, 81, 156), (8, 48, 107),
        ];
        Self::interpolate_palette(&colors, t)
    }

    fn greens(t: f32) -> screen::Rgb {
        let colors = [
            (247, 252, 245), (229, 245, 224), (199, 233, 192), (161, 217, 155),
            (116, 196, 118), (65, 171, 93), (35, 139, 69), (0, 109, 44), (0, 68, 27),
        ];
        Self::interpolate_palette(&colors, t)
    }

    fn reds(t: f32) -> screen::Rgb {
        let colors = [
            (255, 245, 240), (254, 224, 210), (252, 187, 161), (252, 146, 114),
            (251, 106, 74), (239, 59, 44), (203, 24, 29), (165, 15, 21), (103, 0, 13),
        ];
        Self::interpolate_palette(&colors, t)
    }

    fn oranges(t: f32) -> screen::Rgb {
        let colors = [
            (255, 245, 235), (254, 230, 206), (253, 208, 162), (253, 174, 107),
            (253, 141, 60), (241, 105, 19), (217, 72, 1), (166, 54, 3), (127, 39, 4),
        ];
        Self::interpolate_palette(&colors, t)
    }

    fn purples(t: f32) -> screen::Rgb {
        let colors = [
            (252, 251, 253), (239, 237, 245), (218, 218, 235), (188, 189, 220),
            (158, 154, 200), (128, 125, 186), (106, 81, 163), (84, 39, 143), (63, 0, 125),
        ];
        Self::interpolate_palette(&colors, t)
    }

    fn viridis(t: f32) -> screen::Rgb {
        let colors = [
            (68, 1, 84), (72, 40, 120), (62, 74, 137), (49, 104, 142),
            (38, 130, 142), (31, 158, 137), (53, 183, 121), (109, 205, 89),
            (180, 222, 44), (253, 231, 37),
        ];
        Self::interpolate_palette(&colors, t)
    }

    fn plasma(t: f32) -> screen::Rgb {
        let colors = [
            (13, 8, 135), (75, 3, 161), (125, 3, 168), (168, 34, 150),
            (203, 70, 121), (229, 107, 93), (248, 148, 65), (253, 195, 40),
            (240, 249, 33),
        ];
        Self::interpolate_palette(&colors, t)
    }

    fn magma(t: f32) -> screen::Rgb {
        let colors = [
            (0, 0, 4), (28, 16, 68), (79, 18, 123), (129, 37, 129),
            (181, 54, 122), (229, 80, 100), (251, 135, 97), (254, 194, 135),
            (252, 253, 191),
        ];
        Self::interpolate_palette(&colors, t)
    }

    fn inferno(t: f32) -> screen::Rgb {
        let colors = [
            (0, 0, 4), (40, 11, 84), (101, 21, 110), (159, 42, 99),
            (212, 72, 66), (245, 125, 21), (250, 193, 39), (252, 255, 164),
        ];
        Self::interpolate_palette(&colors, t)
    }

    fn coolwarm(t: f32) -> screen::Rgb {
        let colors = [
            (59, 76, 192), (98, 130, 234), (141, 176, 254), (184, 208, 249),
            (221, 221, 221), (245, 196, 173), (244, 154, 123), (222, 96, 77),
            (180, 4, 38),
        ];
        Self::interpolate_palette(&colors, t)
    }

    fn spectral(t: f32) -> screen::Rgb {
        let colors = [
            (158, 1, 66), (213, 62, 79), (244, 109, 67), (253, 174, 97),
            (254, 224, 139), (255, 255, 191), (230, 245, 152), (171, 221, 164),
            (102, 194, 165), (50, 136, 189), (94, 79, 162),
        ];
        Self::interpolate_palette(&colors, t)
    }

    fn interpolate_palette(colors: &[(u8, u8, u8)], t: f32) -> screen::Rgb {
        let n = colors.len();
        let idx = t * (n - 1) as f32;
        let i = (idx.floor() as usize).min(n - 2);
        let frac = idx - i as f32;

        let (r1, g1, b1) = colors[i];
        let (r2, g2, b2) = colors[i + 1];

        screen::Rgb::new(
            (r1 as f32 + frac * (r2 as f32 - r1 as f32)) as u8,
            (g1 as f32 + frac * (g2 as f32 - g1 as f32)) as u8,
            (b1 as f32 + frac * (b2 as f32 - b1 as f32)) as u8,
        )
    }
}

fn graceful_close() -> ! {
    cleanup_terminal();
    exit(0)
}

fn cleanup_terminal() {
    let _ = terminal::disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        crossterm::style::SetAttribute(crossterm::style::Attribute::Reset),
        cursor::Show,
        event::DisableMouseCapture,
        terminal::LeaveAlternateScreen,
    );
    // Also print reset sequence directly in case execute fails
    print!("\x1b[0m\x1b[?25h");
    let _ = io::stdout().flush();
}

fn error_close(msg: &str) -> ! {
    eprintln!("{}", msg);
    exit(1)
}

enum Command {
    View(ViewArgs),
    Search(String),
    CacheInfo,
    CacheClear,
}

struct ViewArgs {
    inputs: Vec<String>,  // Multiple inputs supported
    chain: Option<String>,
    color_scheme: ColorScheme,
}

fn parse_args() -> Option<Command> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return None;
    }

    if ["-h", "-help", "--h", "--help", "help"].contains(&args[1].as_str()) {
        print!("{}", HELP_MSG);
        exit(0);
    }

    if ["-v", "-version", "--v", "--version", "version"].contains(&args[1].as_str()) {
        println!("pepterm {}", env!("CARGO_PKG_VERSION"));
        exit(0);
    }

    if args[1] == "search" {
        if args.len() < 3 {
            error_close("Usage: pepterm search <query>");
        }
        let query = args[2..].join(" ");
        return Some(Command::Search(query));
    }

    if args[1] == "cache" {
        if args.len() >= 3 && args[2] == "clear" {
            return Some(Command::CacheClear);
        } else {
            return Some(Command::CacheInfo);
        }
    }

    let mut inputs = Vec::new();
    let mut color_scheme = ColorScheme::Coolwarm;
    let mut chain: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--color" | "-c" => {
                if i + 1 < args.len() {
                    match ColorScheme::from_str(&args[i + 1]) {
                        Some(scheme) => color_scheme = scheme,
                        None => {
                            error_close(&format!("Unknown color scheme: {}. Use --help for available options.", args[i + 1]));
                        }
                    }
                    i += 2;
                } else {
                    error_close("--color requires a scheme name. Use --help for available options.");
                }
            }
            "--chain" | "-n" => {
                if i + 1 < args.len() {
                    chain = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    error_close("--chain requires a chain ID (e.g., A, B).");
                }
            }
            arg if arg.starts_with('-') => {
                error_close(&format!("Unknown option: {}. Use --help for usage.", arg));
            }
            _ => {
                inputs.push(args[i].clone());
                i += 1;
            }
        }
    }

    if inputs.is_empty() {
        return None;
    }

    Some(Command::View(ViewArgs { inputs, chain, color_scheme }))
}

fn run_search(query: &str) {
    eprintln!("Searching RCSB PDB for '{}'...", query);

    match model::search_pdb(query) {
        Ok(results) => {
            if results.is_empty() {
                println!("No results found for '{}'", query);
            } else {
                println!("\n\x1b[1mSearch Results:\x1b[0m\n");
                for result in &results {
                    let title = if result.title.len() > 60 {
                        format!("{}...", &result.title[..57])
                    } else {
                        result.title.clone()
                    };
                    println!("  \x1b[1;36m{}\x1b[0m  {}", result.pdb_id, title);
                }
                println!("\nUse: pepterm <PDB_ID> to view a structure");
            }
        }
        Err(e) => {
            error_close(&format!("Search failed: {}", e));
        }
    }
}

fn main() {
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        cleanup_terminal();
        default_panic(info);
    }));

    let command = match parse_args() {
        Some(cmd) => cmd,
        None => {
            print!("{}", HELP_MSG);
            exit(0);
        }
    };

    let args = match command {
        Command::Search(query) => {
            run_search(&query);
            exit(0);
        }
        Command::CacheInfo => {
            match model::cache_info() {
                Ok((count, size, path)) => {
                    let size_mb = size as f64 / 1024.0 / 1024.0;
                    println!("Cache directory: {}", path.display());
                    println!("Files: {}", count);
                    println!("Total size: {:.2} MB", size_mb);
                    println!("\nUse 'pepterm cache clear' to remove cached files.");
                }
                Err(e) => error_close(&format!("Failed to get cache info: {}", e)),
            }
            exit(0);
        }
        Command::CacheClear => {
            match model::cache_clear() {
                Ok(count) => {
                    println!("Cleared {} cached files.", count);
                }
                Err(e) => error_close(&format!("Failed to clear cache: {}", e)),
            }
            exit(0);
        }
        Command::View(args) => args,
    };

    let mut color_scheme = args.color_scheme;
    let num_models = args.inputs.len();

    let mut models: Vec<model::Model> = Vec::new();
    let mut model_diagonals: Vec<f32> = Vec::new();
    let mut model_centers: Vec<three::Point> = Vec::new();

    for input in args.inputs.iter() {
        let chain_info = match &args.chain {
            Some(c) => format!(" (chain {})", c),
            None => String::new(),
        };
        eprintln!("Loading {}{}...", input, chain_info);

        match model::new_cartoon(input, args.chain.as_deref(), three::Point::new(0., 0., 0.)) {
            Ok(mut m) => {
                m.apply_color_scheme(|t| color_scheme.get_color(t));

                let bounds = m.world_bounds();
                let center = three::Point::new(
                    (bounds.0.x + bounds.1.x) / 2.,
                    (bounds.0.y + bounds.1.y) / 2.,
                    (bounds.0.z + bounds.1.z) / 2.,
                );
                let diagonal = (
                    (bounds.0.x - bounds.1.x).powi(2) +
                    (bounds.0.y - bounds.1.y).powi(2) +
                    (bounds.0.z - bounds.1.z).powi(2)
                ).sqrt();

                model_centers.push(center);
                model_diagonals.push(diagonal);
                models.push(m);
            }
            Err(error) => {
                error_close(&format!("Error loading {}: {}", input, error));
            }
        }
    }

    terminal::enable_raw_mode().unwrap();
    execute!(
        io::stdout(),
        terminal::EnterAlternateScreen,
        cursor::Hide,
        event::EnableMouseCapture,
        terminal::Clear(terminal::ClearType::All),
    ).unwrap();

    let max_diagonal = model_diagonals.iter().cloned().fold(0.0f32, f32::max);

    let mut camera = three::Camera::new(
        three::Point::new(0., 0., 0.),
        0., 0., 0.,
        VIEWPORT_DISTANCE, VIEWPORT_FOV,
    );

    let initial_yaw: f32 = 0.3;
    let initial_pitch: f32 = 0.2;
    let initial_distance = max_diagonal * INITIAL_DISTANCE_MULTIPLIER;

    let mut view_yaw: f32 = initial_yaw;
    let mut view_pitch: f32 = initial_pitch;
    let mut distance_to_model = initial_distance;
    let mut pan_center = model_centers.get(0).cloned().unwrap_or(three::Point::new(0., 0., 0.));
    let mut pan_mode = false;
    let mut auto_rotate = true;

    let mut mouse_speed: (f32, f32) = (0., 0.);
    let mut last_mouse_position = screen::Point::new(0, 0);
    let mut last_frame_time = TARGET_DURATION_PER_FRAME;

    camera.screen.fit_to_terminal::<screen::BrailePixel>();
    camera.screen.clear();
    thread::sleep(Duration::from_millis(50));

    loop {
        let frame_start = time::Instant::now();
        let mut start_mouse_position = last_mouse_position;
        let mut event_count = 0;

        while event::poll(Duration::from_secs(0)).unwrap() {
            if let Ok(event) = event::read() {
                match event {
                    event::Event::Key(key_event) => {
                        let is_ctrl_c = key_event.modifiers == event::KeyModifiers::CONTROL
                            && key_event.code == event::KeyCode::Char('c');

                        if is_ctrl_c || key_event.code == event::KeyCode::Char('q') {
                            graceful_close()
                        }
                        if key_event.code == event::KeyCode::Char('c') {
                            color_scheme = color_scheme.next();
                            for m in &mut models {
                                m.apply_color_scheme(|t| color_scheme.get_color(t));
                            }
                        }
                        if key_event.code == event::KeyCode::Char('r') {
                            auto_rotate = !auto_rotate;
                        }
                        if key_event.code == event::KeyCode::Char('0') {
                            view_yaw = initial_yaw;
                            view_pitch = initial_pitch;
                            distance_to_model = initial_distance;
                            pan_center = model_centers.get(0).cloned().unwrap_or(three::Point::new(0., 0., 0.));
                            auto_rotate = true;
                        }
                    }

                    event::Event::Mouse(mouse_event) => {
                        let (x, y) = (mouse_event.column, mouse_event.row);
                        match mouse_event.kind {
                            event::MouseEventKind::Down(_) => {
                                pan_mode = mouse_event.modifiers == event::KeyModifiers::SHIFT;
                                last_mouse_position.x = x as i32;
                                last_mouse_position.y = y as i32;
                                start_mouse_position = last_mouse_position;
                                event_count += 1;
                            }

                            event::MouseEventKind::Drag(_) => {
                                pan_mode = mouse_event.modifiers == event::KeyModifiers::SHIFT;
                                if !pan_mode {
                                    auto_rotate = false;
                                }
                                let delta_x = x as f32 - start_mouse_position.x as f32;
                                let delta_y = start_mouse_position.y as f32 - y as f32;
                                mouse_speed.0 = delta_x / camera.screen.width as f32 * MOUSE_SPEED_MULTIPLIER;
                                mouse_speed.1 = delta_y / camera.screen.width as f32 * MOUSE_SPEED_MULTIPLIER;
                                last_mouse_position.x = x as i32;
                                last_mouse_position.y = y as i32;
                                event_count += 1;
                            }

                            event::MouseEventKind::ScrollDown => {
                                distance_to_model += max_diagonal * SCROLL_MULTIPLER;
                            }

                            event::MouseEventKind::ScrollUp => {
                                distance_to_model -= max_diagonal * SCROLL_MULTIPLER;
                                distance_to_model = distance_to_model.max(0.);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        if event_count == 0 {
            mouse_speed = (0., 0.);
            pan_mode = false;
        }

        if pan_mode {
            pan_center.x -= mouse_speed.0 * camera.yaw.cos() * max_diagonal * PAN_MULTIPLIER;
            pan_center.z += mouse_speed.0 * camera.yaw.sin() * max_diagonal * PAN_MULTIPLIER;
            pan_center.y -= mouse_speed.1 * camera.pitch.cos() * max_diagonal * PAN_MULTIPLIER;
            pan_center.x += mouse_speed.1 * camera.yaw.sin() * camera.pitch.sin() * max_diagonal * PAN_MULTIPLIER;
            pan_center.z += mouse_speed.1 * camera.yaw.cos() * camera.pitch.sin() * max_diagonal * PAN_MULTIPLIER;
        } else if auto_rotate {
            view_yaw += AUTO_ROTATE_SPEED;
        } else {
            view_yaw -= mouse_speed.0;
            view_pitch -= mouse_speed.1;
        }

        camera.screen.fit_to_terminal::<screen::BrailePixel>();
        camera.screen.clear();

        let calc_camera_pos = |center: &three::Point, dist: f32| -> three::Point {
            three::Point::new(
                view_yaw.sin() * view_pitch.cos() * dist + center.x,
                view_pitch.sin() * dist + center.y,
                -view_yaw.cos() * view_pitch.cos() * dist + center.z,
            )
        };

        if num_models == 1 {
            let cam_pos = calc_camera_pos(&pan_center, distance_to_model);
            camera.coordinates = cam_pos;
            camera.yaw = -view_yaw;
            camera.pitch = -view_pitch;
            camera.plot_model_colored_edges(&models[0]);
        } else {
            let viewport_width = camera.screen.width / num_models as u16;
            let full_height = camera.screen.height;
            let limiting_size = (viewport_width as f32).min(full_height as f32 / 2.0);
            let scale_factor = limiting_size * 0.012;

            for (i, model) in models.iter().enumerate() {
                let base_distance = model_diagonals[i] * INITIAL_DISTANCE_MULTIPLIER * scale_factor;
                let model_distance = base_distance * (distance_to_model / initial_distance);

                camera.plot_model_in_viewport(
                    model,
                    calc_camera_pos(&model_centers[i], model_distance),
                    -view_yaw,
                    -view_pitch,
                    i as u16 * viewport_width,
                    viewport_width,
                    full_height,
                );
            }
        }

        let rotate_msg = if auto_rotate { "auto" } else { "manual" };
        let fps = 1. / last_frame_time.as_secs_f32();
        let input_display = if args.inputs.len() == 1 {
            args.inputs[0].clone()
        } else if args.inputs.len() <= 4 {
            args.inputs.join("+")
        } else {
            format!("{} structures", args.inputs.len())
        };

        let status_full = format!(
            "{} | {} | {} | {:.0}fps | [r]otate [c]olor [0]reset [q]uit",
            input_display, color_scheme.name(), rotate_msg, fps
        );
        let status_medium = format!(
            "{} | {} | {} | {:.0}fps",
            input_display, color_scheme.name(), rotate_msg, fps
        );
        let status_short = format!("{} | {}", input_display, color_scheme.name());

        let final_msg = match terminal::size().unwrap().0 as usize {
            w if w > status_full.len() => status_full,
            w if w > status_medium.len() => status_medium,
            w if w > status_short.len() => status_short,
            _ => String::new(),
        };

        camera.screen.render_with_status::<screen::BrailePixel>(&final_msg);

        let elapsed = frame_start.elapsed();
        if elapsed < TARGET_DURATION_PER_FRAME {
            thread::sleep(TARGET_DURATION_PER_FRAME - elapsed);
        }
        last_frame_time = frame_start.elapsed();
    }
}
