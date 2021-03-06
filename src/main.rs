#![deny(overflowing_literals, unsafe_code)]
#![deny(warnings)]
#![allow(
    unknown_lints,
    match_wild_err_arm,
    too_many_arguments,
    cyclomatic_complexity,
    expect_fun_call,
    or_fun_call,
    unused_macros
)]
#![windows_subsystem = "windows"]

// NOTE: the external functions must be available in crate root:
#[cfg(feature = "web")]
pub use crate::engine::wasm::{initialise, key_pressed, update};

mod ai;
mod animation;
mod blocker;
mod color;
mod engine;
#[macro_use]
mod error;
mod formula;
mod game;
mod generators;
mod graphics;
mod item;
mod keys;
mod level;
mod metadata;
mod monster;
mod palette;
mod pathfinding;
mod player;
mod point;
mod random;
mod ranged_int;
mod rect;
mod render;
mod state;
mod stats;
mod timer;
mod ui;
mod util;
mod window;
mod windows;
mod world;

// These are all in tiles and relate to how much we show on the screen.
//
// NOTE: 53 x 30 tiles is the same aspect ratio as a widescreen
// monitor. So that's the ideal to strive for. But if we want to keep
// the display map square, the sidebar gets too wide.
//
// So instead, we've narrowed the sidebar to 17 tiles (just enough to
// make every withdrawal step show up). That means we don't maintain
// the perfect aspect ratio, but it seems to be good enough.
const DISPLAYED_MAP_SIZE: i32 = 30;
const PANEL_WIDTH: i32 = 17;
const DISPLAY_SIZE: point::Point = point::Point {
    x: DISPLAYED_MAP_SIZE + PANEL_WIDTH,
    y: DISPLAYED_MAP_SIZE,
};
const WORLD_SIZE: point::Point = point::Point {
    x: 1_073_741_824,
    y: 1_073_741_824,
};

#[allow(unused_variables, dead_code, needless_pass_by_value)]
fn run_glutin(
    display_size: point::Point,
    default_background: color::Color,
    window_title: &str,
    state: state::State,
    update: engine::UpdateFn,
) {
    log::info!("Using the glutin backend");

    // // TODO: figure out how to record screenshots with glutin!
    // let (fixed_fps, replay_dir) = if record_replay {
    //     (Some(60), Some("/home/thomas/tmp/dose-response-recording"))
    // } else {
    //     (None, None)
    // };

    #[cfg(feature = "glutin-backend")]
    engine::glutin::main_loop(
        display_size,
        default_background,
        window_title,
        Box::new(state),
        update,
    );

    #[cfg(not(feature = "glutin-backend"))]
    log::error!("The \"glutin-backend\" feature was not compiled in.");
}

#[allow(unused_variables, dead_code)]
fn run_sdl(
    display_size: point::Point,
    default_background: color::Color,
    window_title: &str,
    state: state::State,
    update: engine::UpdateFn,
) {
    log::info!("Using the sdl backend");

    #[cfg(feature = "sdl-backend")]
    engine::sdl::main_loop(
        display_size,
        default_background,
        window_title,
        Box::new(state),
        update,
    );

    #[cfg(not(feature = "sdl-backend"))]
    log::error!("The \"sdl-backend\" feature was not compiled in.");
}

#[allow(unused_variables, dead_code, needless_pass_by_value)]
fn run_remote(
    display_size: point::Point,
    default_background: color::Color,
    window_title: &str,
    state: state::State,
    update: engine::UpdateFn,
) {
    #[cfg(feature = "remote")]
    engine::remote::main_loop(
        display_size,
        default_background,
        window_title,
        Box::new(state),
        update,
    );

    #[cfg(not(feature = "remote"))]
    log::error!("The \"remote\" feature was not compiled in.");
}

#[cfg(feature = "cli")]
fn process_cli_and_run_game() {
    use clap::{App, Arg, ArgGroup};
    use simplelog::{CombinedLogger, Config, LevelFilter, SharedLogger, SimpleLogger, WriteLogger};
    use std::fs::File;

    let mut app = App::new(metadata::TITLE)
        .version(metadata::VERSION)
        .author(metadata::AUTHORS)
        .about(metadata::DESCRIPTION)
        .arg(
            Arg::with_name("exit-after")
                .help("Exit after the game or replay has finished")
                .long("exit-after"),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .help("Don't write any messages to stdout."),
        )
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .help("Print debug-level info. This can be really verbose."),
        );

    if cfg!(feature = "cheating") {
        app = app
            .arg(
                Arg::with_name("cheating")
                    .help("Opens the cheat mode on start. Uncovers the map.")
                    .long("cheating"),
            )
            .arg(
                Arg::with_name("invincible")
                    .help("Makes the player character invincible. They do not die.")
                    .long("invincible"),
            );
    }

    // if cfg!(feature = "remote") {
    //     app = app.arg(Arg::with_name("remote").long("remote").help(
    //         "Don't create a game window. The input and output is \
    //          controled via ZeroMQ.",
    //     ));
    //     graphics_backends.push("remote");
    // }

    if cfg!(feature = "glutin-backend") {
        app = app.arg(
            Arg::with_name("glutin")
                .long("glutin")
                .help("Use the glutin rendering backend"),
        );
        if !crate::engine::AVAILABLE_BACKENDS.contains(&"glutin") {
            log::error!("The `glutin` backend is enabled, but not set by the build script?");
        }
    }

    if cfg!(feature = "sdl-backend") {
        app = app.arg(
            Arg::with_name("sdl")
                .long("sdl")
                .help("Use the SDL2 rendering backend"),
        );
        if !crate::engine::AVAILABLE_BACKENDS.contains(&"sdl") {
            log::error!("The `sdl` backend is enabled, but not set by the build script?");
        }
    }

    // Make sure only one of the backends can be set at a time
    app = app.group(ArgGroup::with_name("graphics").args(&crate::engine::AVAILABLE_BACKENDS));

    if cfg!(feature = "replay") {
        app = app
            // NOTE: this is a positional argument, because it doesn't
            // have `.short` or `.long` set. It looks similar to a
            // "keyword" arguments that take values (such as
            // --replay-file) but it's different.
            .arg(
                Arg::with_name("replay")
                    .value_name("FILE")
                    .help(
                        "Replay this file instead of starting and playing a new \
                         game",
                    )
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("replay-full-speed")
                    .help(
                        "Don't slow the replay down (useful for getting accurate \
                         measurements)",
                    )
                    .long("replay-full-speed"),
            )
            .arg(
                Arg::with_name("replay-file")
                    .help("Path where to store the replay log.")
                    .long("replay-file")
                    .value_name("FILE")
                    .takes_value(true),
            );
    }

    if cfg!(feature = "recording") {
        app = app.arg(
            Arg::with_name("record-frames")
                .long("record-frames")
                .help("Whether to record the frames and save them on disk."),
        );
    }

    let matches = app.get_matches();

    let default_graphics_backend = if crate::engine::AVAILABLE_BACKENDS.contains(&"glutin") {
        "glutin"
    } else {
        crate::engine::AVAILABLE_BACKENDS[0]
    };
    assert!(crate::engine::AVAILABLE_BACKENDS.contains(&default_graphics_backend));

    let mut loggers = vec![];

    let log_level = if matches.is_present("debug") {
        LevelFilter::Trace
    } else {
        LevelFilter::Info
    };

    if !matches.is_present("quiet") {
        loggers.push(SimpleLogger::new(log_level, Config::default()) as Box<dyn SharedLogger>);
    }

    if let Ok(logfile) = File::create("dose-response.log") {
        loggers.push(WriteLogger::new(log_level, Config::default(), logfile));
    }

    // NOTE: ignore the loggers if we can't initialise them. The game
    // should still be able to function.
    let _ = CombinedLogger::init(loggers);

    log::info!("{} version: {}", metadata::TITLE, metadata::VERSION);
    log::info!("By: {}", metadata::AUTHORS);
    log::info!("{}", metadata::HOMEPAGE);

    let hash = metadata::GIT_HASH.trim();
    if !hash.is_empty() {
        log::info!("Git commit: {}", hash);
    }
    let target_triple = metadata::TARGET_TRIPLE.trim();
    if !target_triple.is_empty() {
        log::info!("Target triple: {}", target_triple);
    }

    log::info!(
        "Available font sizes: {:?}",
        crate::engine::AVAILABLE_FONT_SIZES
    );

    log::info!(
        "Available graphics backends: {:?}",
        crate::engine::AVAILABLE_BACKENDS
    );

    // NOTE: Generate the default settings file contents
    let mut settings = String::with_capacity(1000);
    settings.push_str("# Options: \"fullscreen\" or \"window\"\n");
    settings.push_str("display = \"window\"\n\n");

    let font_sizes_str = crate::engine::AVAILABLE_FONT_SIZES
        .iter()
        .map(|num| num.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    settings.push_str(&format!("# Options: {}\n", font_sizes_str));
    settings.push_str(&format!("font_size = {}\n\n", crate::engine::TILESIZE));

    let backends_str = crate::engine::AVAILABLE_BACKENDS
        .iter()
        .map(|b| format!("\"{}\"", b))
        .collect::<Vec<_>>()
        .join(", ");
    settings.push_str(&format!("# Options: {}\n", backends_str));
    settings.push_str(&format!("backend = \"{}\"\n", default_graphics_backend));
    log::info!("Default settings:");
    println!("{}", settings);

    let loaded_settings = settings
        .parse::<toml_edit::Document>()
        .expect("Couldn't load settings.");
    log::info!("Loaded settings:");
    println!("{}", loaded_settings.to_string());

    log::info!(
        "display: {:?}",
        loaded_settings["display"]
            .as_str()
            .expect("The `display` setting must be a string.")
    );
    log::info!(
        "font size: {:?}",
        loaded_settings["font_size"]
            .as_integer()
            .expect("The `font_size` setting must be an integer.")
    );
    log::info!(
        "graphics backend: {:?}",
        loaded_settings["backend"]
            .as_str()
            .expect("The `backend` setting must be a string.")
    );

    let state = if let Some(replay) = matches.value_of("replay") {
        if matches.is_present("replay-file") {
            panic!(
                "The `replay-file` option can only be used during regular \
                 game, not replay."
            );
        }
        let replay_path = std::path::Path::new(replay);
        state::State::replay_game(
            WORLD_SIZE,
            DISPLAYED_MAP_SIZE,
            PANEL_WIDTH,
            DISPLAY_SIZE,
            &replay_path,
            matches.is_present("cheating"),
            matches.is_present("invincible"),
            matches.is_present("replay-full-speed"),
            matches.is_present("exit-after"),
        )
        .expect("Could not load the replay file")
    } else {
        if matches.is_present("replay-full-speed") {
            panic!(
                "The `full-replay-speed` option can only be used if the \
                 replay log is passed."
            );
        }
        let replay_file = match matches.value_of("replay-file") {
            Some(file) => Some(file.into()),
            None => state::generate_replay_path(),
        };
        let mut state = state::State::new_game(
            WORLD_SIZE,
            DISPLAYED_MAP_SIZE,
            PANEL_WIDTH,
            DISPLAY_SIZE,
            matches.is_present("exit-after"),
            replay_file,
            matches.is_present("invincible"),
        );
        state.window_stack.push(window::Window::MainMenu);
        state.first_game_already_generated = true;
        state
    };

    let display_size = DISPLAY_SIZE;
    let background = color::unexplored_background;
    let game_title = metadata::TITLE;
    let game_update = game::update;

    let backend = if matches.is_present("remote") {
        "remote"
    } else if matches.is_present("sdl") {
        "sdl"
    } else if matches.is_present("glutin") {
        "glutin"
    } else {
        default_graphics_backend
    };

    match backend {
        "remote" => run_remote(display_size, background, game_title, state, game_update),
        "sdl" => run_sdl(display_size, background, game_title, state, game_update),
        "glutin" => run_glutin(display_size, background, game_title, state, game_update),
        _ => {
            log::error!("Unknown backend: {}", backend);
        }
    }
}

// NOTE: this function is intentionally empty and should stay here.
// Under wasm we don't want to run the game immediately because the
// game loop must be controlled from the browser not Rust. So we've
// provided external endpoints the browser will call in. But `main`
// still gets executed when the wasm binary is loaded.
#[cfg(not(feature = "cli"))]
fn process_cli_and_run_game() {}

fn main() {
    process_cli_and_run_game();
}
