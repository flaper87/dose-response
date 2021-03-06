use crate::{
    animation::{AreaOfEffect, ScreenFade},
    engine::Mouse,
    keys::Keys,
    monster,
    player::Player,
    point::Point,
    random::{self, Random},
    stats::Stats,
    timer::Timer,
    util,
    window::Window,
    windows,
    world::{MonsterId, World},
};

use std::{
    collections::VecDeque,
    error::Error,
    fs::File,
    io::{self, Write},
    path::Path,
    path::PathBuf,
    time::Duration,
};

#[cfg(feature = "replay")]
use std::fs;

use serde::{Deserialize, Serialize};

// TODO: Rename this to `GameState` and the existing `GameState` to
// `Game`? It's no longer just who's side it is but also: did the
// player won? Lost?
#[derive(Copy, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Side {
    Player,
    Victory,
}

// TODO: rename this to Input or something like that. This represents the raw
// commands from the player or AI abstracted from keyboard, joystick or
// whatever. But they shouldn't carry any context or data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Command {
    N,
    E,
    S,
    W,
    NE,
    NW,
    SE,
    SW,
    UseFood,
    UseDose,
    UseCardinalDose,
    UseDiagonalDose,
    UseStrongDose,
    ShowMessageBox { ttl: Duration, message: String },
}

#[cfg(feature = "replay")]
pub fn generate_replay_path() -> Option<PathBuf> {
    use chrono::prelude::*;
    let local_time = Local::now();

    // Timestamp in format: 2016-11-20T20-04-39.123. We can't use the
    // colons in the timestamp -- Windows don't allow them in a path.
    let timestamp = local_time.format("%FT%H-%M-%S%.3f");
    let replay_dir = &Path::new("replays");
    assert!(replay_dir.is_relative());
    if !replay_dir.exists() {
        fs::create_dir_all(replay_dir).unwrap();
    }
    let replay_path = &replay_dir.join(format!("replay-{}", timestamp));
    Some(replay_path.into())
}

#[cfg(not(feature = "replay"))]
pub fn generate_replay_path() -> Option<PathBuf> {
    None
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Verification {
    pub turn: i32,
    pub chunk_count: usize,
    pub player_pos: Point,
    pub monsters: Vec<(Point, Point, monster::Kind)>,
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub player: Player,
    #[serde(skip_serializing, skip_deserializing)]
    pub explosion_animation: Option<Box<dyn AreaOfEffect>>,

    /// The actual size of the game world in tiles. Could be infinite
    /// but we're limiting it for performance reasons for now.
    pub world_size: Point,
    pub chunk_size: i32,
    pub world: World,

    /// The size of the game map inside the game window. We're keeping
    /// this square so this value repesents both width and heigh.
    /// It's a window into the game world that is actually rendered.
    pub map_size: Point,

    /// The width of the in-game status panel.
    pub panel_width: i32,

    /// The size of the game window in tiles. The area stuff is
    /// rendered to. NOTE: currently, the width is equal to map_size +
    /// panel_width, height is map_size.
    pub display_size: Point,
    pub screen_position_in_world: Point,
    pub seed: u32,
    pub rng: Random,
    pub keys: Keys,
    pub mouse: Mouse,
    pub commands: VecDeque<Command>,
    #[serde(skip_serializing, skip_deserializing)]
    pub verifications: VecDeque<Verification>,
    #[serde(skip_serializing, skip_deserializing, default = "empty_command_logger")]
    pub command_logger: Box<dyn Write>,
    pub side: Side,
    pub turn: i32,
    pub cheating: bool,
    pub replay: bool,
    pub replay_full_speed: bool,
    pub exit_after: bool,
    pub clock: Duration,
    pub replay_step: Duration,
    #[serde(skip_serializing, skip_deserializing)]
    pub stats: Stats,
    pub pos_timer: Timer,
    pub paused: bool,
    pub old_screen_pos: Point,
    pub new_screen_pos: Point,
    pub screen_fading: Option<ScreenFade>,
    pub offset_px: Point,

    /// Whether the game is over (one way or another) and we should
    /// show the endgame screen -- uncovered map, the score, etc.
    pub game_ended: bool,
    pub victory_npc_id: Option<MonsterId>,

    pub window_stack: windows::Windows<Window>,

    pub first_game_already_generated: bool,
    pub show_keboard_movement_hints: bool,
    pub show_anxiety_counter: bool,
    pub current_help_window: windows::help::Page,

    /// Whether we should push the Endscreen window and uncover the
    /// map during the transition from screen fade out to fade in
    /// phase. This is purely a visual effect and the values here are
    /// a bit of a hack. If there's more instances of us wanting to do
    /// this, we hould just have a list of screen fade transition
    /// effects here instead.
    pub show_endscreen_and_uncover_map_during_fadein: bool,
    pub uncovered_map: bool,
}

impl State {
    fn new<W: Write + 'static>(
        world_size: Point,
        map_size: i32,
        panel_width: i32,
        display_size: Point,
        commands: VecDeque<Command>,
        verifications: VecDeque<Verification>,
        log_writer: W,
        seed: u32,
        cheating: bool,
        invincible: bool,
        replay: bool,
        replay_full_speed: bool,
        exit_after: bool,
    ) -> State {
        let world_centre = (0, 0).into();
        assert_eq!(world_size.x, world_size.y);
        assert_eq!(display_size, (map_size + panel_width, map_size));
        let player_position = world_centre;
        let player = Player::new(player_position, invincible);
        let mut rng = random::from_seed(u64::from(seed));
        let world = World::new(&mut rng, seed, world_size.x, 32, player.info());

        State {
            player,
            explosion_animation: None,
            chunk_size: 32,
            world_size,
            world,
            map_size: (map_size, map_size).into(),
            panel_width,
            display_size,
            screen_position_in_world: world_centre,
            seed,
            rng,
            keys: Keys::new(),
            mouse: Default::default(),
            commands,
            verifications,
            command_logger: Box::new(log_writer),
            side: Side::Player,
            turn: 0,
            cheating,
            replay,
            replay_full_speed,
            exit_after,
            clock: Duration::new(0, 0),
            replay_step: Duration::new(0, 0),
            stats: Default::default(),
            pos_timer: Timer::new(Duration::from_millis(0)),
            old_screen_pos: (0, 0).into(),
            new_screen_pos: (0, 0).into(),
            offset_px: Point::zero(),
            paused: false,
            screen_fading: None,
            game_ended: false,
            victory_npc_id: None,
            window_stack: windows::Windows::new(Window::Game),
            first_game_already_generated: false,
            show_keboard_movement_hints: true,
            show_anxiety_counter: false,
            current_help_window: windows::help::Page::DoseResponse,
            show_endscreen_and_uncover_map_during_fadein: false,
            uncovered_map: false,
        }
    }

    pub fn new_game(
        world_size: Point,
        map_size: i32,
        panel_width: i32,
        display_size: Point,
        exit_after: bool,
        replay_path: Option<PathBuf>,
        invincible: bool,
    ) -> State {
        let commands = VecDeque::new();
        let verifications = VecDeque::new();
        let seed = util::random_seed();
        let mut writer: Box<dyn Write> = if let Some(replay_path) = replay_path {
            match File::create(&replay_path) {
                Ok(f) => {
                    log::info!("Recording the gameplay to '{}'", replay_path.display());
                    Box::new(f)
                }
                Err(msg) => panic!(
                    "Failed to create the replay file at '{:?}'.
Reason: '{}'.",
                    replay_path.display(),
                    msg
                ),
            }
        } else {
            Box::new(io::sink())
        };

        log_header(&mut writer, seed);
        let cheating = false;
        let replay = false;
        let replay_full_speed = false;
        State::new(
            world_size,
            map_size,
            panel_width,
            display_size,
            commands,
            verifications,
            writer,
            seed,
            cheating,
            invincible,
            replay,
            replay_full_speed,
            exit_after,
        )
    }

    #[cfg(not(feature = "replay"))]
    #[allow(dead_code)]
    pub fn replay_game(
        world_size: Point,
        map_size: i32,
        panel_width: i32,
        display_size: Point,
        _replay_path: &Path,
        _cheating: bool,
        invincible: bool,
        _replay_full_speed: bool,
        exit_after: bool,
    ) -> Result<State, Box<dyn Error>> {
        Ok(Self::new_game(
            world_size,
            map_size,
            panel_width,
            display_size,
            exit_after,
            None,
            invincible,
        ))
    }

    #[cfg(feature = "replay")]
    pub fn replay_game(
        world_size: Point,
        map_size: i32,
        panel_width: i32,
        display_size: Point,
        replay_path: &Path,
        cheating: bool,
        invincible: bool,
        replay_full_speed: bool,
        exit_after: bool,
    ) -> Result<State, Box<dyn Error>> {
        use serde_json;
        use std::io::{BufRead, BufReader};
        let mut commands = VecDeque::new();
        let mut verifications = VecDeque::new();
        let seed: u32;
        let file = File::open(replay_path)?;
        let mut lines = BufReader::new(file).lines();
        match lines.next() {
            Some(seed_str) => seed = seed_str?.parse()?,
            None => error!("The replay file is empty."),
        };

        match lines.next() {
            Some(version) => {
                let version = version?;
                if version != crate::metadata::VERSION {
                    log::warn!(
                        "The replay file's version is: {}, but the program is: {}",
                        version,
                        crate::metadata::VERSION
                    );
                }
            }
            None => error!("The replay file is missing the version."),
        };

        match lines.next() {
            Some(commit) => {
                let commit = commit?;
                if commit != crate::metadata::GIT_HASH {
                    log::warn!(
                        "The replay file's commit is: {}, but the program is: {}.",
                        commit,
                        crate::metadata::GIT_HASH
                    );
                }
            }
            None => error!("The replay file is missing the commit hash."),
        };

        loop {
            match lines.next() {
                Some(line) => {
                    let line = line?;
                    let command = serde_json::from_str(&line);
                    // Try parsing it as a command, otherwise it's a verification
                    if let Ok(command) = command {
                        commands.push_back(command);
                    } else {
                        let verification = serde_json::from_str(&line)?;
                        verifications.push_back(verification);
                    }
                }
                None => break,
            }
        }

        log::info!("Replaying game log: '{}'", replay_path.display());
        let cheating = cheating;
        let invincible = invincible;
        let replay = true;
        Ok(State::new(
            world_size,
            map_size,
            panel_width,
            display_size,
            commands,
            verifications,
            Box::new(io::sink()),
            seed,
            cheating,
            invincible,
            replay,
            replay_full_speed,
            exit_after,
        ))
    }

    pub fn verification(&self) -> Verification {
        // TODO: we can sort the chunks and compare directly at some point.
        let chunks = self.world.positions_of_all_chunks();
        let mut monsters = vec![];
        for &chunk_pos in &chunks {
            for monster in self.world.chunk(chunk_pos).unwrap().monsters() {
                if !monster.dead {
                    monsters.push((monster.position, chunk_pos, monster.kind));
                }
            }
        }
        monsters
            .sort_by_key(|&(monster_pos, _chunk_pos, kind)| (monster_pos.x, monster_pos.y, kind));

        Verification {
            turn: self.turn,
            chunk_count: chunks.len(),
            player_pos: self.player.pos,
            monsters,
        }
    }

    pub fn save_to_file(&self) -> Result<(), Box<dyn Error>> {
        // TODO: select the filename dynamicaly!
        let filename = "SAVEDGAME.sav";
        let version_data = bincode::serialize(crate::metadata::VERSION)?;
        let commit_data = bincode::serialize(crate::metadata::GIT_HASH)?;
        let state_data = bincode::serialize(self)?;

        // TODO: this can be compressed nicely!

        let mut file = File::create(filename)?;
        file.write_all(&version_data)?;
        file.write_all(&commit_data)?;
        file.write_all(&state_data)?;
        file.flush()?;

        Ok(())
    }

    pub fn load_from_file() -> Result<State, Box<dyn Error>> {
        let filename = "SAVEDGAME.sav";
        let state = {
            let file = File::open(filename)?;
            let version: String = bincode::deserialize_from(&file)?;
            log::info!("Savefile version {}", version);
            if version != crate::metadata::VERSION {
                log::warn!("The game was saved in a different version: {}. This release has version: {}. The game might not load properly.",
                           version,
                           crate::metadata::VERSION);
            }
            let commit: String = bincode::deserialize_from(&file)?;
            log::info!("Savefile commit {}", commit);
            if commit != crate::metadata::GIT_HASH {
                log::warn!("The game was saved in a different commit: {}. This release has commit: {}. The game might not load properly.",
                           commit,
                crate::metadata::GIT_HASH);
            }
            bincode::deserialize_from(&file)?
        };

        if let Err(error) = ::std::fs::remove_file(filename) {
            log::error!(
                "Failed to delete the successfully loaded savegame. Error: {:?}",
                error
            );
        }

        Ok(state)
    }
}

fn empty_command_logger() -> Box<dyn Write> {
    Box::new(io::sink())
}

pub fn log_header<W: Write>(writer: &mut W, seed: u32) {
    writeln!(writer, "{}", seed).unwrap();
    writeln!(writer, "{}", crate::metadata::VERSION).unwrap();
    writeln!(writer, "{}", crate::metadata::GIT_HASH).unwrap();
}

pub fn log_command<W: Write>(writer: &mut W, command: Command) {
    use serde_json;
    let json_command = serde_json::to_string(&command).expect(&format!(
        "Could not \
         serialise {:?} to \
         json.",
        command
    ));
    writeln!(writer, "{}", json_command).unwrap();
}

pub fn log_verification<W: Write>(writer: &mut W, verification: &Verification) {
    use serde_json;
    let json = serde_json::to_string(&verification).expect(&format!(
        "Could not \
         serialise \
         {:?} to json.",
        verification
    ));
    writeln!(writer, "{}", json).expect(&format!(
        "Could not write the \
         verification: '{}' to the \
         replay log.",
        json
    ));
}
