#![cfg(feature = "sdl")]

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::Color;

use crate::engine::{
    bindings, combos_internal, current_state_info, display_for_internal, engine_from_gmr_file,
    matched_prefix_len, step_keytok, EngineConfig, EngineState, print_engine
};

#[derive(Debug, Clone)]
enum AppEvent {
    KeyTok(String),
    Quit,
}

type NowMs = u128;

#[derive(Debug, Clone)]
struct ViewState {
    engine: EngineState,
    recent_msgs: VecDeque<String>,
}

fn keycode_letter(kc: Keycode) -> Option<char> {
    Some(match kc {
        Keycode::A=>'a', Keycode::B=>'b', Keycode::C=>'c', Keycode::D=>'d', Keycode::E=>'e',
        Keycode::F=>'f', Keycode::G=>'g', Keycode::H=>'h', Keycode::I=>'i', Keycode::J=>'j',
        Keycode::K=>'k', Keycode::L=>'l', Keycode::M=>'m', Keycode::N=>'n', Keycode::O=>'o',
        Keycode::P=>'p', Keycode::Q=>'q', Keycode::R=>'r', Keycode::S=>'s', Keycode::T=>'t',
        Keycode::U=>'u', Keycode::V=>'v', Keycode::W=>'w', Keycode::X=>'x', Keycode::Y=>'y',
        Keycode::Z=>'z',
        _ => return None,
    })
}

fn keytok_from_sdl(key: Keycode, km: Mod) -> Option<String> {
    let mut pref = String::new();
    let has = |m: Mod| km.intersects(m);
    if has(Mod::LSHIFTMOD) || has(Mod::RSHIFTMOD) { pref.push_str("shift-"); }
    if has(Mod::LALTMOD)   || has(Mod::RALTMOD)   { pref.push_str("alt-"); }
    if has(Mod::LCTRLMOD)  || has(Mod::RCTRLMOD)  { pref.push_str("ctrl-"); }

    let base = match key {
        Keycode::Up        => "up".into(),
        Keycode::Down      => "down".into(),
        Keycode::Left      => "left".into(),
        Keycode::Right     => "right".into(),
        Keycode::Space     => "space".into(),
        Keycode::Return    => "enter".into(),
        Keycode::Backspace => "backspace".into(),
        Keycode::Delete    => "delete".into(),
        kc => keycode_letter(kc)?.to_string(),
    };

    Some(format!("{pref}{base}"))
}

fn map_sdl_event(ev: Event) -> Option<AppEvent> {
    match ev {
        Event::Quit { .. } => Some(AppEvent::Quit),
        Event::KeyDown { keycode: Some(kc), keymod, repeat, .. } if !repeat => {
            /* check if esc or ctrl+c */
            if kc == Keycode::Escape {
                return Some(AppEvent::Quit);
            }
            if let Some(tok) = keytok_from_sdl(kc, keymod) {
                if tok == "ctrl-c" { return Some(AppEvent::Quit); }
                Some(AppEvent::KeyTok(tok))
            } else {
                None
            }
        }
        _ => None,
    }
}

/* (cfg, state, event, now) -> new state */
fn reduce(cfg: &EngineConfig, vs: &ViewState, ev: AppEvent, now_ms: NowMs) -> ViewState {
    match ev {
        AppEvent::Quit => vs.clone(),
        AppEvent::KeyTok(tok) => {
            let (engine2, outs) = step_keytok(cfg, vs.engine, &tok, now_ms);
            let mut msgs = vs.recent_msgs.clone();
            for m in outs {
                if msgs.len() >= 8 { msgs.pop_front(); }
                msgs.push_back(m);
            }
            ViewState { engine: engine2, recent_msgs: msgs }
        }
    }
}

#[derive(Clone)]
struct UiLine { text: String, rgb: (u8, u8, u8) }

#[derive(Clone)]
struct UiModel {
    left_title: UiLine,
    left_bindings: Vec<UiLine>,
    combos_title: UiLine,
    combos_lines: Vec<UiLine>,
    right_title: UiLine,
    cur_state_line: UiLine,
    fail_line: UiLine,
    outs_title: UiLine,
    outs_lines: Vec<UiLine>,
    recent_title: UiLine,
    recent_lines: Vec<UiLine>,
    footer: UiLine,
}

fn build_ui_model(cfg: &EngineConfig, st: &ViewState) -> UiModel {
    let col_norm  = (220, 220, 220);
    let col_hit   = (160, 240, 200);
    let col_bind  = (230, 230, 230);
    let col_title_l = (200, 200, 255);
    let col_title_r = (200, 255, 200);
    let col_sub    = (200, 200, 200);
    let col_out    = (255, 215, 130);
    let col_recent = (255, 255, 160);
    let col_footer = (160, 160, 160);

    let left_bindings: Vec<UiLine> = bindings(cfg)
        .iter()
        .map(|(key, internal)| UiLine { text: format!("{:>12}  →  {}", key, internal), rgb: col_bind })
        .collect();

    let combos_lines: Vec<UiLine> = combos_internal(cfg)
        .iter()
        .map(|(steps, mv)| {
            let prefix_len = matched_prefix_len(cfg, st.engine.cur_state, steps);
            let mut line = String::new();
            for (i, internal) in steps.iter().enumerate() {
                if i > 0 { line.push_str(" , "); }
                let lbl = display_for_internal(cfg, internal);
                line.push_str(&lbl);
            }
            line.push_str("  =>  ");
            line.push_str(mv);
            UiLine { text: line, rgb: if prefix_len > 0 { col_hit } else { col_norm } }
        })
        .collect();

    let (outs_now, fail) = current_state_info(cfg, st.engine);

    UiModel {
        left_title: UiLine { text: "Keyboard bindings:".to_string(), rgb: col_title_l },
        left_bindings,
        combos_title: UiLine { text: "Available combos:".to_string(), rgb: col_title_l },
        combos_lines,
        right_title: UiLine { text: "Automaton".to_string(), rgb: col_title_r },
        cur_state_line: UiLine { text: format!("Current state: {}", st.engine.cur_state), rgb: col_norm },
        fail_line: UiLine { text: format!("Fail link: {}", fail), rgb: col_sub },
        outs_title: UiLine { text: "Outputs at state:".to_string(), rgb: col_sub },
        outs_lines: outs_now.into_iter().map(|o| UiLine { text: format!("• {}", o), rgb: col_out }).collect(),
        recent_title: UiLine { text: "Recent:".to_string(), rgb: col_sub },
        recent_lines: st.recent_msgs.iter().cloned().map(|m| UiLine { text: m, rgb: col_recent }).collect(),
        footer: UiLine { text: "Exit: Esc o ctrl-c".to_string(), rgb: col_footer },
    }
}

#[derive(Clone)]
struct TextNode { x: i32, y: i32, line: UiLine }

#[derive(Clone)]
struct Scene { bg: (u8, u8, u8), texts: Vec<TextNode> }

fn layout_scene(ui: &UiModel, font_h: i32) -> Scene {
    let left_x: i32 = 16;
    let right_x: i32 = 520;
    let h_total: i32 = 600;
    let line_h: i32 = font_h.max(16) + 6;

    let mut texts = Vec::new();

    /* left panel */
    texts.push(TextNode { x: left_x, y: 14, line: ui.left_title.clone() });
    let mut y = 14 + 28;
    for l in &ui.left_bindings {
        texts.push(TextNode { x: left_x, y, line: l.clone() });
        y += line_h;
    }
    texts.push(TextNode { x: left_x, y: y + 20, line: ui.combos_title.clone() });
    let mut yc = y + 20 + 28;
    for l in &ui.combos_lines {
        texts.push(TextNode { x: left_x, y: yc, line: l.clone() });
        yc += line_h;
    }

    /* right panel */
    texts.push(TextNode { x: right_x, y: 14, line: ui.right_title.clone() });
    texts.push(TextNode { x: right_x, y: 40, line: ui.cur_state_line.clone() });
    texts.push(TextNode { x: right_x, y: 68, line: ui.fail_line.clone() });
    texts.push(TextNode { x: right_x, y: 96, line: ui.outs_title.clone() });
    let mut y2 = 120;
    for l in &ui.outs_lines {
        texts.push(TextNode { x: right_x + 20, y: y2, line: l.clone() });
        y2 += line_h;
    }

    texts.push(TextNode { x: right_x, y: 220, line: ui.recent_title.clone() });
    let mut y3 = 244;
    for l in &ui.recent_lines {
        texts.push(TextNode { x: right_x + 20, y: y3, line: l.clone() });
        y3 += line_h;
    }

    texts.push(TextNode { x: right_x - 150, y: h_total - 28, line: ui.footer.clone() });

    Scene { bg: (18, 18, 18), texts }
}

pub fn run_sdl(
    path: &str,
    debug: bool,
    step_timeout_ms: u64,
    font_path: &str,
) -> Result<(), String> {
    let (cfg, st0) = engine_from_gmr_file(path, Duration::from_millis(step_timeout_ms))?;

    print_engine(&cfg);

    let sdl = sdl2::init().map_err(|e| e.to_string())?;
    let video = sdl.video().map_err(|e| e.to_string())?;
    let ttf = sdl2::ttf::init().map_err(|e| e.to_string())?;
    let font = ttf.load_font(font_path, 18).map_err(|e| e.to_string())?;

    let window = video
        .window("ft_ality (SDL GUI)", 900, 600)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();

    let draw_text = {
        let font = &font;
        let texture_creator = &texture_creator;
        move |canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
              x: i32, y: i32, text: &str, rgb: (u8,u8,u8)|
              -> Result<(), String> {
            let color = Color::RGB(rgb.0, rgb.1, rgb.2);
            let surface = font.render(text).blended(color).map_err(|e| e.to_string())?;
            let texture = texture_creator.create_texture_from_surface(&surface).map_err(|e| e.to_string())?;
            let rect = sdl2::rect::Rect::new(x, y, surface.width(), surface.height());
            canvas.copy(&texture, None, rect)?;
            Ok(())
        }
    };

    let mut view = ViewState { engine: st0, recent_msgs: VecDeque::new() };
    let mut event_pump = sdl.event_pump().map_err(|e| e.to_string())?;
    let start = Instant::now();

    'mainloop: loop {
        let mut evs: Vec<AppEvent> = Vec::new();
        for ev in event_pump.poll_iter() {
            if let Some(ae) = map_sdl_event(ev) {
                evs.push(ae);
            }
        }
        let should_quit = evs.iter().any(|e| matches!(e, AppEvent::Quit));

        let now_ms: NowMs = start.elapsed().as_millis() as u128;
        view = evs.into_iter().fold(view, |acc, e| reduce(&cfg, &acc, e, now_ms));

        let ui = build_ui_model(&cfg, &view);
        let scene = layout_scene(&ui, font.height() as i32);

        let (r, g, b) = scene.bg;
        canvas.set_draw_color(Color::RGB(r, g, b));
        canvas.clear();
        for node in &scene.texts {
            (draw_text)(&mut canvas, node.x, node.y, &node.line.text, node.line.rgb)?;
        }
        canvas.present();

        if debug {
            eprintln!("[state={}]", view.engine.cur_state);
        }

        if should_quit { break 'mainloop; }
    }

    Ok(())
}
