#![cfg(feature = "sdl")]

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::Color;

use crate::engine::{Engine};

fn keycode_letter(kc: Keycode) -> Option<char> {
    Some(match kc {
        Keycode::A=>'a', Keycode::B=>'b', Keycode::C=>'c', Keycode::D=>'d', Keycode::E=>'e', Keycode::F=>'f', Keycode::G=>'g',
        Keycode::H=>'h', Keycode::I=>'i', Keycode::J=>'j', Keycode::K=>'k', Keycode::L=>'l', Keycode::M=>'m', Keycode::N=>'n',
        Keycode::O=>'o', Keycode::P=>'p', Keycode::Q=>'q', Keycode::R=>'r', Keycode::S=>'s', Keycode::T=>'t', Keycode::U=>'u',
        Keycode::V=>'v', Keycode::W=>'w', Keycode::X=>'x', Keycode::Y=>'y', Keycode::Z=>'z',
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

pub fn run_sdl(path: &str, debug: bool, step_timeout_ms: u64, font_path: &str) -> Result<(), String> {
    let mut eng = Engine::from_gmr_file(path, Duration::from_millis(step_timeout_ms))?;

    /* Setup */
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

    let measure_text = {
        let font = &font;
        move |text: &str| -> Result<(u32, u32), String> {
            let surf = font.render(text).blended(Color::WHITE).map_err(|e| e.to_string())?;
            Ok((surf.width(), surf.height()))
        }
    };

    let draw_text = {
        let font = &font;
        let texture_creator = &texture_creator;
        move |canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
              x: i32, y: i32, text: &str, color: Color|
              -> Result<(), String> {
            let surface = font.render(text).blended(color).map_err(|e| e.to_string())?;
            let texture = texture_creator.create_texture_from_surface(&surface).map_err(|e| e.to_string())?;
            let rect = sdl2::rect::Rect::new(x, y, surface.width(), surface.height());
            canvas.copy(&texture, None, rect)?;
            Ok(())
        }
    };

    /* Layout constants */
    let left_x: i32 = 16;
    let right_x: i32 = 520; /* Right panel */
    let panel_w_left: i32 = 480 - 32;
    let h_total: i32 = 600;

    /* scroll */
    let mut scroll_bindings: i32 = 0;
    let mut scroll_combos: i32 = 0;
    let mut mouse_x: i32 = 0;
    let mut mouse_y: i32 = 0;
    let line_h: i32 = (font.height() as i32).max(16) + 6;

    /* recent outputs */
    let mut recent_msgs: VecDeque<String> = VecDeque::new();

    let mut event_pump = sdl.event_pump().map_err(|e| e.to_string())?;
    'mainloop: loop {
        /* Event handling */
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} => break 'mainloop,
                Event::MouseMotion { x, y, .. } => { mouse_x = x; mouse_y = y; }
                Event::MouseWheel { y, .. } => {
                    /* wheel up/down scrolls the panel under the mouse */
                    let in_bindings = mouse_x >= left_x && mouse_x < right_x && mouse_y < 300;
                    let in_combos   = mouse_x >= left_x && mouse_x < right_x && mouse_y >= 300 && mouse_y < h_total-20;
                    let delta = (-y) * line_h;
                    if in_bindings { scroll_bindings = (scroll_bindings + delta).max(0); }
                    if in_combos   { scroll_combos   = (scroll_combos   + delta).max(0); }
                }
                Event::KeyDown { keycode: Some(kc), keymod, repeat, .. } if !repeat => {
                    /* PageUp/PageDown scrolling */
                    match kc {
                        Keycode::PageDown => { if mouse_x < right_x { scroll_bindings += 8*line_h; } else { scroll_combos += 8*line_h; } }
                        Keycode::PageUp   => { if mouse_x < right_x { scroll_bindings = (scroll_bindings - 8*line_h).max(0); } else { scroll_combos = (scroll_combos - 8*line_h).max(0); } }
                        _ => {}
                    }

                    if let Some(keytok) = keytok_from_sdl(kc, keymod) {
                        /* Exit */
                        if keytok == "ctrl-c" || (kc == Keycode::Escape && keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD)) {
                            break 'mainloop;
                        }

                        let outs = eng.step_keytok(&keytok, Instant::now());
                        for m in outs {
                            if recent_msgs.len() >= 8 { recent_msgs.pop_front(); }
                            recent_msgs.push_back(m);
                        }
                        if debug {
                            println!("{keytok}  ⇒  state={}", eng.current_state());
                        }
                    }
                }
                _ => {}
            }
        }

        /* start drawing */
        canvas.set_draw_color(Color::RGB(18, 18, 18));
        canvas.clear();

        /* (Right) Key bindings */
        let bindings_top = 14 - scroll_bindings + 28; // Initial y position for bindings
        (draw_text)(&mut canvas, left_x, 14 - scroll_bindings, "Keyboard bindings:", Color::RGB(200, 200, 255))?;
        
        eng.bindings()
            .iter()
            .enumerate()
            .for_each(|(i, (key, internal))| {
                let y = bindings_top + (i as i32) * line_h;
                let line = format!("{:>12}  →  {}", key, internal);
                let _ = (draw_text)(&mut canvas, left_x, y, &line, Color::RGB(230, 230, 230));
            });

        /* Scroll */
        let bindings_content_h = (bindings_top + ((eng.bindings().len() as i32) * line_h)) - 14;
        let max_scroll_bindings = (bindings_content_h - (300 - 14)).max(0);
        if scroll_bindings > max_scroll_bindings { scroll_bindings = max_scroll_bindings; }

        /* (Left) Combos */
        let mut yc = 314 - scroll_combos;
        (draw_text)(&mut canvas, left_x, 300 - scroll_combos, "Available combos:", Color::RGB(200, 200, 255))?;

        let col_norm  = Color::RGB(220, 220, 220);
        let col_sep   = Color::RGB(160, 160, 160);
        let col_hit   = Color::RGB(160, 240, 200);
        let col_move  = Color::RGB(255, 235, 140);

        let draw_segments_wrapped = |canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
                                     start_x: i32,
                                     mut yline: i32,
                                     max_w: i32,
                                     segs: &[(String, Color)]|
            -> Result<i32, String> {
            let mut x = start_x;
            let mut consumed_h = line_h;
            for (text, color) in segs {
                let (w, _) = measure_text(text)?;
                if x + (w as i32) <= start_x + max_w {
                    (draw_text)(canvas, x, yline, text, *color)?;
                    x += w as i32;
                    continue;
                }
                let mut chunk = String::new();
                let mut last_break = 0usize;
                let chars: Vec<char> = text.chars().collect();
                for (i, ch) in chars.iter().enumerate() {
                    chunk.push(*ch);
                    let (cw, _) = measure_text(&chunk)?;
                    let fits = x + (cw as i32) <= start_x + max_w;
                    let is_break = *ch == ' ' || *ch == '/' || *ch == ',';
                    if !fits {
                        /* wrap to next line */
                        yline += line_h;
                        consumed_h += line_h;
                        x = start_x;
                        /* draw previous chunk without the char that overflowed */
                        let prev = &text[last_break..i];
                        if !prev.is_empty() { (draw_text)(canvas, x, yline, prev, *color)?; }
                        x += measure_text(prev)?.0 as i32;
                        chunk.clear();
                        chunk.push(*ch);
                        last_break = i;
                    } else if is_break {
                        /* draw up to here and continue */
                        let piece = &text[last_break..=i];
                        (draw_text)(canvas, x, yline, piece, *color)?;
                        x += measure_text(piece)?.0 as i32;
                        last_break = i + 1;
                        chunk.clear();
                    }
                }
                /* remainder */
                let tail = &text[last_break..];
                if !tail.is_empty() {
                    let (tw, _) = measure_text(tail)?;
                    if x + (tw as i32) > start_x + max_w {
                        /* wrap once more if needed */
                        yline += line_h;
                        consumed_h += line_h;
                        x = start_x;
                    }
                    (draw_text)(canvas, x, yline, tail, *color)?;
                    x += tw as i32;
                }
            }
            Ok(consumed_h)
        };

        let combos_area_bottom = h_total - 20;
        let combos_top = 300;

        /* Build all the combos internal. */
        for (steps, mv) in eng.combos_internal().iter() {
            let prefix_len = eng.matched_prefix_len(steps);
            let mut segs: Vec<(String, Color)> = Vec::new();

            for (i, internal) in steps.iter().enumerate() {
                if i > 0 { segs.push((" , ".to_string(), col_sep)); }
                let label = eng.display_for_internal(internal);
                let color = if i < prefix_len { col_hit } else { col_norm };
                segs.push((label, color));
            }
            segs.push(("  =>  ".to_string(), col_sep));
            segs.push((mv.clone(), col_move));

            let sim_y = yc;
            let height_used = draw_segments_wrapped(&mut canvas, left_x, sim_y, panel_w_left, &segs)?;
            yc += height_used;
            if yc > combos_area_bottom - scroll_combos { break; }
        }
        let combos_content_h = (yc - (300 - scroll_combos)).max(0);
        let max_scroll_combos = (combos_content_h - (combos_area_bottom - combos_top)).max(0);
        if scroll_combos > max_scroll_combos { scroll_combos = max_scroll_combos; }

        (draw_text)(&mut canvas, right_x, 14, "Automaton", Color::RGB(200, 255, 200))?;
        (draw_text)(&mut canvas, right_x, 40, &format!("Current state: {}", eng.current_state()), Color::RGB(230, 230, 230))?;
        let (outs_now, fail) = eng.current_state_info();
        (draw_text)(&mut canvas, right_x, 68, &format!("Fail link: {}", fail), Color::RGB(200, 200, 200))?;
        (draw_text)(&mut canvas, right_x, 96, "Outputs at state:", Color::RGB(200, 200, 200))?;
        let mut y2 = 120;
        for o in &outs_now {
            (draw_text)(&mut canvas, right_x + 20, y2, &format!("• {}", o), Color::RGB(255, 215, 130))?;
            y2 += line_h;
        }

        (draw_text)(&mut canvas, right_x, 220, "Recent:", Color::RGB(200, 200, 200))?;
        let mut y3 = 244;
        for m in &recent_msgs {
            (draw_text)(&mut canvas, right_x + 20, y3, &format!("{}", m), Color::RGB(255, 255, 160))?;
            y3 += line_h;
        }

        /* Footer */
        (draw_text)(&mut canvas, right_x - 150, h_total - 28, "Scroll: mouse wheel / PgUp/PgDn • Exit: Ctrl+Esc or ctrl-c", Color::RGB(160, 160, 160))?;

        canvas.present();
    }

    Ok(())
}
