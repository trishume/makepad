use std::collections::HashMap;
use std::collections::BTreeMap;

pub use crate::shadergen::*;
pub use crate::cx_fonts::*;
pub use crate::cx_turtle::*;
pub use crate::cx_cursor::*;
pub use crate::cx_window::*;
pub use crate::cx_view::*;
pub use crate::cx_pass::*;
pub use crate::cx_texture::*;
pub use crate::cx_shader::*;
pub use crate::math::*;
pub use crate::events::*;
pub use crate::colors::*;
pub use crate::elements::*;
pub use crate::animator::*;
pub use crate::area::*;

#[cfg(target_os = "linux")]
pub use crate::cx_ogl::*;

#[cfg(target_os = "macos")]
pub use crate::cx_mtl::*;

#[cfg(target_os = "macos")]
pub use crate::cx_mtlsl::*;

#[cfg(target_os = "windows")]
pub use crate::cx_dx11::*;

#[cfg(target_os = "windows")]
pub use crate::cx_hlsl::*;

#[cfg(target_arch = "wasm32")]
pub use crate::cx_webgl::*;

#[cfg(any(target_arch = "wasm32", target_os = "linux"))]
pub use crate::cx_glsl::*;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use crate::cx_desktop::*;

pub struct Cx {
    pub title: String,
    pub running: bool,
    pub is_desktop_build: bool,
    
    pub fonts: Vec<CxFont>,
    
    pub textures: Vec<CxTexture>,
    pub textures_free: Vec<usize>,
    
    pub views: Vec<CxView>,
    pub views_free: Vec<usize>,
    pub view_stack: Vec<usize>,
    
    pub window_stack: Vec<usize>,
    pub windows: Vec<CxWindow>,
    pub windows_free: Vec<usize>,
    
    pub pass_stack: Vec<usize>,
    pub passes: Vec<CxPass>,
    pub passes_free: Vec<usize>,
    
    //pub current_draw_list_id: Option<usize>,
    
    pub shaders: Vec<CxShader>,
    pub shader_map: HashMap<CxShader, usize>,
    
    pub redraw_child_areas: Vec<Area>,
    pub redraw_parent_areas: Vec<Area>,
    pub _redraw_child_areas: Vec<Area>,
    pub _redraw_parent_areas: Vec<Area>,
    
    //pub paint_dirty2: bool,
    pub clear_color: Color,
    pub redraw_id: u64,
    pub repaint_id: u64,
    pub event_id: u64,
    pub timer_id: u64,
    pub signal_id: u64,
    pub is_in_redraw_cycle: bool,
    
    pub last_key_focus: Area,
    pub key_focus: Area,
    pub keys_down: Vec<KeyEvent>,
    
    pub debug_area: Area,
    
    pub turtles: Vec<Turtle>,
    pub align_list: Vec<Area>,
    
    //pub window_geom: WindowGeom,
    
    pub down_mouse_cursor: Option<MouseCursor>,
    pub hover_mouse_cursor: Option<MouseCursor>,
    pub captured_fingers: Vec<Area>,
    pub finger_tap_count: Vec<(Vec2, f64, u32)>,
    
    //pub user_events:Vec<Event>,
    
    pub playing_anim_areas: Vec<AnimArea>,
    pub ended_anim_areas: Vec<AnimArea>,
    
    pub frame_callbacks: Vec<Area>,
    pub _frame_callbacks: Vec<Area>,
    
    pub signals_before_draw: Vec<(Signal, u64)>,
    pub signals_after_draw: Vec<(Signal, u64)>,
    
    pub platform: CxPlatform,
    
    pub style_values: BTreeMap<String, StyleValue>,
    
    pub panic_now: bool,
    pub panic_redraw: bool
}


impl Default for Cx {
    fn default() -> Self {
        let mut captured_fingers = Vec::new();
        let mut finger_tap_count = Vec::new();
        for _i in 0..10 {
            captured_fingers.push(Area::Empty);
            finger_tap_count.push((Vec2::zero(), 0.0, 0));
        }
        
        Self {
            title: "Hello World".to_string(),
            is_desktop_build: false,
            running: true,
            
            fonts: Vec::new(),
            
            textures: Vec::new(),
            textures_free: Vec::new(),
            
            views: vec![CxView {..Default::default()}],
            views_free: Vec::new(),
            view_stack: Vec::new(),
            
            window_stack: Vec::new(),
            windows: Vec::new(),
            windows_free: Vec::new(),
            
            pass_stack: Vec::new(),
            passes: Vec::new(),
            passes_free: Vec::new(),
            
            shaders: Vec::new(),
            shader_map: HashMap::new(),
            
            redraw_parent_areas: Vec::new(),
            _redraw_parent_areas: Vec::new(),
            redraw_child_areas: Vec::new(),
            _redraw_child_areas: Vec::new(),
            //paint_dirty: false,
            clear_color: Color {r: 0.1, g: 0.1, b: 0.1, a: 1.0},
            
            redraw_id: 1,
            event_id: 1,
            repaint_id: 1,
            timer_id: 1,
            signal_id: 1,
            
            is_in_redraw_cycle: false,
            turtles: Vec::new(),
            align_list: Vec::new(),
            
            //window_geom: WindowGeom {..Default::default()},
            
            last_key_focus: Area::Empty,
            key_focus: Area::Empty,
            keys_down: Vec::new(),
            
            debug_area: Area::Empty,
            
            down_mouse_cursor: None,
            hover_mouse_cursor: None,
            captured_fingers: captured_fingers,
            finger_tap_count: finger_tap_count,
            
            //user_events:Vec::new(),
            
            style_values: BTreeMap::new(),
            
            playing_anim_areas: Vec::new(),
            ended_anim_areas: Vec::new(),
            
            frame_callbacks: Vec::new(),
            _frame_callbacks: Vec::new(),
            
            //custom_before_draw:Vec::new(),
            signals_before_draw: Vec::new(),
            signals_after_draw: Vec::new(),
            
            platform: CxPlatform {..Default::default()},
            
            panic_now: false,
            panic_redraw: false
        }
    }
}


impl Cx {
    pub fn new_shader(&mut self) -> CxShader {
        let mut sh = CxShader {..Default::default()};
        CxShader::def_builtins(&mut sh);
        CxShader::def_df(&mut sh);
        CxPass::def_uniforms(&mut sh);
        CxView::def_uniforms(&mut sh);
        sh
    }
    
    //pub fn get_shader2(&self, id: usize) -> &CompiledShader {
    //    &self.compiled_shaders[id]
   // }
    
    pub fn add_shader(&mut self, sh: CxShader, name: &str) -> Shader {
        let next_id = self.shaders.len();
        let store_id = self.shader_map.entry(sh.clone()).or_insert(next_id);
        if *store_id == next_id {
            self.shaders.push(CxShader {
                name: name.to_string(),
                ..sh
            });
        }
        Shader{shader_id:Some(*store_id)}
    }
    
    pub fn process_tap_count(&mut self, digit: usize, pos: Vec2, time: f64) -> u32 {
        if digit >= self.finger_tap_count.len() {
            return 0
        };
        let (last_pos, last_time, count) = self.finger_tap_count[digit];
        
        if (time - last_time) < 0.5 && pos.distance(&last_pos) < 10. {
            self.finger_tap_count[digit] = (pos, time, count + 1);
            count + 1
        }
        else {
            self.finger_tap_count[digit] = (pos, time, 1);
            1
        }
    }
    
    
    pub fn redraw_pass_of(&mut self, area: Area) {
        // we walk up the stack of area
        match area {
            Area::All => {
                for window_id in 0..self.windows.len() {
                    let redraw = match self.windows[window_id].window_state {
                        CxWindowState::Create {..} | CxWindowState::Created => {
                            true
                        },
                        _ => false
                    };
                    if redraw {
                        if let Some(pass_id) = self.windows[window_id].main_pass_id {
                            self.redraw_pass_and_dep_of_passes(pass_id);
                        }
                    }
                }
            },
            Area::Empty => (),
            Area::Instance(instance) => {
                self.redraw_pass_and_dep_of_passes(self.views[instance.view_id].pass_id);
            },
            Area::View(viewarea) => {
                self.redraw_pass_and_dep_of_passes(self.views[viewarea.view_id].pass_id);
            }
        }
    }
    
    pub fn redraw_pass_and_dep_of_passes(&mut self, pass_id: usize) {
        let mut walk_pass_id = pass_id;
        loop {
            if let Some(main_view_id) = self.passes[walk_pass_id].main_view_id {
                self.redraw_parent_area(Area::View(ViewArea {redraw_id: 0, view_id: main_view_id}));
            }
            match self.passes[walk_pass_id].dep_of.clone() {
                CxPassDepOf::Pass(next_pass_id) => {
                    walk_pass_id = next_pass_id;
                },
                _ => {
                    break;
                }
            }
        }
    }
    
    pub fn redraw_pass_and_sub_passes(&mut self, pass_id: usize) {
        let cxpass = &self.passes[pass_id];
        if let Some(main_view_id) = cxpass.main_view_id {
            self.redraw_parent_area(Area::View(ViewArea {redraw_id: 0, view_id: main_view_id}));
        }
        // lets redraw all subpasses as well
        for sub_pass_id in 0..self.passes.len() {
            if let CxPassDepOf::Pass(dep_pass_id) = self.passes[sub_pass_id].dep_of.clone() {
                if dep_pass_id == pass_id {
                    self.redraw_pass_and_sub_passes(sub_pass_id);
                }
            }
        }
    }
    
    pub fn redraw_child_area(&mut self, area: Area) {
        if self.panic_redraw {
            #[cfg(debug_assertions)]
            panic!("Panic Redraw triggered")
        }
        
        // if we are redrawing all, clear the rest
        if area == Area::All {
            self.redraw_child_areas.truncate(0);
        }
        // check if we are already redrawing all
        else if self.redraw_child_areas.len() == 1 && self.redraw_child_areas[0] == Area::All {
            return;
        };
        // only add it if we dont have it already
        if let Some(_) = self.redraw_child_areas.iter().position( | a | *a == area) {
            return;
        }
        self.redraw_child_areas.push(area);
    }
    
    pub fn redraw_parent_area(&mut self, area: Area) {
        if self.panic_redraw {
            #[cfg(debug_assertions)]
            panic!("Panic Redraw triggered")
        }
        
        // if we are redrawing all, clear the rest
        if area == Area::All {
            self.redraw_parent_areas.truncate(0);
        }
        // check if we are already redrawing all
        else if self.redraw_parent_areas.len() == 1 && self.redraw_parent_areas[0] == Area::All {
            return;
        };
        // only add it if we dont have it already
        if let Some(_) = self.redraw_parent_areas.iter().position( | a | *a == area) {
            return;
        }
        self.redraw_parent_areas.push(area);
    }
    
    pub fn redraw_previous_areas(&mut self) {
        for area in self._redraw_child_areas.clone() {
            self.redraw_child_area(area);
        }
        for area in self._redraw_parent_areas.clone() {
            self.redraw_parent_area(area);
        }
    }
    
    pub fn view_will_redraw(&self, view_id: usize) -> bool {
        
        // figure out if areas are in some way a child of draw_list_id, then we need to redraw
        for area in &self._redraw_child_areas {
            match area {
                Area::All => {
                    return true;
                },
                Area::Empty => (),
                Area::Instance(instance) => {
                    let mut vw = instance.view_id;
                    if vw == view_id {
                        return true
                    }
                    while vw != 0 {
                        vw = self.views[vw].nesting_view_id;
                        if vw == view_id {
                            return true
                        }
                    }
                },
                Area::View(viewarea) => {
                    let mut vw = viewarea.view_id;
                    if vw == view_id {
                        return true
                    }
                    while vw != 0 {
                        vw = self.views[vw].nesting_view_id;
                        if vw == view_id {
                            return true
                        }
                    }
                    
                }
            }
        }
        // figure out if areas are in some way a parent of draw_list_id, then redraw
        for area in &self._redraw_parent_areas {
            match area {
                Area::All => {
                    return true;
                },
                Area::Empty => (),
                Area::Instance(instance) => {
                    let mut vw = view_id;
                    if vw == instance.view_id {
                        return true
                    }
                    while vw != 0 {
                        vw = self.views[vw].nesting_view_id;
                        if vw == instance.view_id {
                            return true
                        }
                    }
                },
                Area::View(viewarea) => {
                    let mut vw = view_id;
                    if vw == viewarea.view_id {
                        return true
                    }
                    while vw != 0 {
                        vw = self.views[vw].nesting_view_id;
                        if vw == viewarea.view_id {
                            return true
                        }
                    }
                    
                }
            }
        }
        
        false
    }
    
    
    
    pub fn check_ended_anim_areas(&mut self, time: f64) {
        let mut i = 0;
        self.ended_anim_areas.truncate(0);
        loop {
            if i >= self.playing_anim_areas.len() {
                break
            }
            let anim_start_time = self.playing_anim_areas[i].start_time;
            let anim_total_time = self.playing_anim_areas[i].total_time;
            if anim_start_time.is_nan() || time - anim_start_time >= anim_total_time {
                self.ended_anim_areas.push(self.playing_anim_areas.remove(i));
            }
            else {
                i = i + 1;
            }
        }
    }
    
    pub fn update_area_refs(&mut self, old_area: Area, new_area: Area) {
        if old_area == Area::Empty || old_area == Area::All {
            return
        }
        
        if let Some(anim_anim) = self.playing_anim_areas.iter_mut().find( | v | v.area == old_area) {
            anim_anim.area = new_area.clone()
        }
        
        if let Some(digit_area) = self.captured_fingers.iter_mut().find( | v | **v == old_area) {
            *digit_area = new_area.clone()
        }
        // update capture keyboard
        if self.key_focus == old_area {
            self.key_focus = new_area.clone()
        }
        //
        if let Some(next_frame) = self.frame_callbacks.iter_mut().find( | v | **v == old_area) {
            *next_frame = new_area.clone()
        }
    }
    
    pub fn color(&self, name: &str) -> Color {
        if let Some(StyleValue::Color(val)) = self.style_values.get(name) {
            return *val;
        }
        panic!("Cannot find style color key {}", name);
    }
    
    pub fn font(&self, name: &str) -> String {
        if let Some(StyleValue::Font(val)) = self.style_values.get(name) {
            return val.clone();
        }
        panic!("Cannot find style font key {}", name);
    }
    
    pub fn size(&self, name: &str) -> f64 {
        if let Some(StyleValue::Size(val)) = self.style_values.get(name) {
            return *val;
        }
        panic!("Cannot find style size key {}", name);
    }
    
    pub fn set_color(&mut self, name: &str, val: Color) {
        self.style_values.insert(name.to_string(), StyleValue::Color(val));
    }
    
    pub fn set_font(&mut self, name: &str, val: &str) {
        self.style_values.insert(name.to_string(), StyleValue::Font(val.to_string()));
    }
    
    pub fn set_size(&mut self, name: &str, val: f64) {
        self.style_values.insert(name.to_string(), StyleValue::Size(val));
    }
    
    pub fn set_key_focus(&mut self, focus_area: Area) {
        self.key_focus = focus_area;
    }
    
    pub fn has_key_focus(&self, focus_area: Area) -> bool {
        self.key_focus == focus_area
    }
    
    pub fn process_key_down(&mut self, key_event: KeyEvent) {
        if let Some(_) = self.keys_down.iter().position( | k | k.key_code == key_event.key_code) {
            return;
        }
        self.keys_down.push(key_event);
    }
    
    pub fn process_key_up(&mut self, key_event: &KeyEvent) {
        for i in 0..self.keys_down.len() {
            if self.keys_down[i].key_code == key_event.key_code {
                self.keys_down.remove(i);
                return
            }
        }
    }
    
    pub fn call_all_keys_up<F>(&mut self, mut event_handler: F)
    where F: FnMut(&mut Cx, &mut Event)
    {
        let keys_down = self.keys_down.clone();
        self.keys_down.truncate(0);
        for key_event in keys_down {
            self.call_event_handler(&mut event_handler, &mut Event::KeyUp(key_event))
        }
    }
    
    // event handler wrappers
    
    pub fn call_event_handler<F>(&mut self, mut event_handler: F, event: &mut Event)
    where F: FnMut(&mut Cx, &mut Event)
    {
        self.event_id += 1;
        event_handler(self, event);
        
        if self.last_key_focus != self.key_focus {
            let last_key_focus = self.last_key_focus;
            self.last_key_focus = self.key_focus;
            event_handler(self, &mut Event::KeyFocus(KeyFocusEvent {
                last: last_key_focus,
                focus: self.key_focus
            }))
        }
    }
    
    pub fn call_draw_event<F>(&mut self, mut event_handler: F)
    where F: FnMut(&mut Cx, &mut Event)
    {
        self.is_in_redraw_cycle = true;
        self.redraw_id += 1;
        self._redraw_child_areas = self.redraw_child_areas.clone();
        self._redraw_parent_areas = self.redraw_parent_areas.clone();
        self.align_list.truncate(0);
        self.redraw_child_areas.truncate(0);
        self.redraw_parent_areas.truncate(0);
        self.call_event_handler(&mut event_handler, &mut Event::Draw);
        self.is_in_redraw_cycle = false;
    }
    
    pub fn call_animation_event<F>(&mut self, mut event_handler: F, time: f64)
    where F: FnMut(&mut Cx, &mut Event)
    {
        self.call_event_handler(&mut event_handler, &mut Event::Animate(AnimateEvent {time: time, frame: self.repaint_id}));
        self.check_ended_anim_areas(time);
        if self.ended_anim_areas.len() > 0 {
            self.call_event_handler(&mut event_handler, &mut Event::AnimateEnded(AnimateEvent {time: time, frame: self.repaint_id}));
        }
    }
    
    pub fn call_frame_event<F>(&mut self, mut event_handler: F, time: f64)
    where F: FnMut(&mut Cx, &mut Event)
    {
        self._frame_callbacks = self.frame_callbacks.clone();
        self.frame_callbacks.truncate(0);
        self.call_event_handler(&mut event_handler, &mut Event::Frame(FrameEvent {time: time, frame: self.repaint_id}));
    }
    
    pub fn next_frame(&mut self, area: Area) {
        if let Some(_) = self.frame_callbacks.iter().position( | a | *a == area) {
            return;
        }
        self.frame_callbacks.push(area);
    }
    
    pub fn new_signal(&mut self) -> Signal {
        self.signal_id += 1;
        return Signal {signal_id: self.signal_id}
    }
    
    pub fn send_signal_before_draw(&mut self, signal: Signal, message: u64) {
        self.signals_before_draw.push((signal, message));
    }
    
    pub fn send_signal_after_draw(&mut self, signal: Signal, message: u64) {
        self.signals_after_draw.push((signal, message));
    }
    
    pub fn call_signals_before_draw<F>(&mut self, mut event_handler: F)
    where F: FnMut(&mut Cx, &mut Event)
    {
        if self.signals_before_draw.len() == 0 {
            return
        }
        
        let signals_before_draw = self.signals_before_draw.clone();
        self.signals_before_draw.truncate(0);
        for (signal, value) in signals_before_draw {
            self.call_event_handler(&mut event_handler, &mut Event::Signal(SignalEvent {
                signal_id: signal.signal_id,
                value: value
            }));
        }
    }
    
    pub fn call_signals_after_draw<F>(&mut self, mut event_handler: F)
    where F: FnMut(&mut Cx, &mut Event)
    {
        if self.signals_after_draw.len() == 0 {
            return
        }
        
        let signals_after_draw = self.signals_after_draw.clone();
        self.signals_after_draw.truncate(0);
        for (signal, value) in signals_after_draw {
            self.call_event_handler(&mut event_handler, &mut Event::Signal(SignalEvent {
                signal_id: signal.signal_id,
                value: value
            }));
        }
    }
    
    /*
    pub fn debug_draw_tree_recur(&mut self, draw_list_id: usize, depth:usize){
        if draw_list_id >= self.draw_lists.len(){
            println!("---------- Drawlist still empty ---------");
            return
        }
        let mut indent = String::new();
        for _i in 0..depth{
            indent.push_str("  ");
        }
        let draw_calls_len = self.draw_lists[draw_list_id].draw_calls_len;
        if draw_list_id == 0{
            println!("---------- Begin Debug draw tree for redraw_id: {} ---------", self.redraw_id)
        }
        println!("{}list {}: len:{} rect:{:?}", indent, draw_list_id, draw_calls_len, self.draw_lists[draw_list_id].rect);  
        indent.push_str("  ");
        for draw_call_id in 0..draw_calls_len{
            let sub_list_id = self.draw_lists[draw_list_id].draw_calls[draw_call_id].sub_list_id;
            if sub_list_id != 0{
                self.debug_draw_tree_recur(sub_list_id, depth + 1);
            }
            else{
                let draw_list = &mut self.draw_lists[draw_list_id];
                let draw_call = &mut draw_list.draw_calls[draw_call_id];
                let sh = &self.shaders[draw_call.shader_id];
                let shc = &self.compiled_shaders[draw_call.shader_id];
                let slots = shc.instance_slots;
                let instances = draw_call.instance.len() / slots;
                println!("{}call {}: {}({}) x:{}", indent, draw_call_id, sh.name, draw_call.shader_id, instances);  
                // lets dump the instance geometry
                for inst in 0..instances.min(1){
                    let mut out = String::new();
                    let mut off = 0;
                    for prop in &shc.named_instance_props.props{
                        match prop.slots{
                            1=>out.push_str(&format!("{}:{} ", prop.name,
                                draw_call.instance[inst*slots + off])),
                            2=>out.push_str(&format!("{}:v2({},{}) ", prop.name,
                                draw_call.instance[inst*slots+ off],
                                draw_call.instance[inst*slots+1+ off])),
                            3=>out.push_str(&format!("{}:v3({},{},{}) ", prop.name,
                                draw_call.instance[inst*slots+ off],
                                draw_call.instance[inst*slots+1+ off],
                                draw_call.instance[inst*slots+1+ off])),
                            4=>out.push_str(&format!("{}:v4({},{},{},{}) ", prop.name,
                                draw_call.instance[inst*slots+ off],
                                draw_call.instance[inst*slots+1+ off],
                                draw_call.instance[inst*slots+2+ off],
                                draw_call.instance[inst*slots+3+ off])),
                            _=>{}
                        }
                        off += prop.slots;
                    }
                    println!("  {}instance {}: {}", indent, inst, out);  
                }
            }
        }
        if draw_list_id == 0{
            println!("---------- End Debug draw tree for redraw_id: {} ---------", self.redraw_id)
        }
    }*/
}


#[derive(Clone)]
pub enum StyleValue {
    Color(Color),
    Font(String),
    Size(f64)
}

pub trait Style {
    fn style(cx: &mut Cx) -> Self;
}

#[macro_export]
macro_rules!log {
    ( $ ( $ arg: tt) *) => ({
        $ crate::Cx::write_log(&format!("[{}:{}:{}] {}\n", file!(), line!(), column!(), &format!( $ ( $ arg) *)))
    })
}

#[macro_export]
macro_rules!main_app {
    ( $ app: ident, $ name: expr) => {
        //TODO do this with a macro to generate both entrypoints for App and Cx
        pub fn main() {
            let mut cx = Cx {
                title: $ name.to_string(),
                ..Default::default()
            };
            
            let mut app = $ app {
                ..Style::style(&mut cx)
            };
            
            cx.event_loop( | cx, mut event | {
                if let Event::Draw = event {return app.draw_app(cx);}
                app.handle_app(cx, &mut event);
            });
        }
        
        #[export_name = "create_wasm_app"]
        pub extern "C" fn create_wasm_app() -> u32 {
            let mut cx = Box::new(
                Cx {
                    title: $ name.to_string(),
                    ..Default::default()
                }
            );
            let app = Box::new(
                $ app {
                    ..Style::style(&mut cx)
                }
            );
            Box::into_raw(Box::new((Box::into_raw(app), Box::into_raw(cx)))) as u32
        }
        
        #[export_name = "process_to_wasm"]
        pub unsafe extern "C" fn process_to_wasm(appcx: u32, msg_bytes: u32) -> u32 {
            let appcx = &*(appcx as *mut (*mut $ app, *mut Cx));
            (*appcx.1).process_to_wasm(msg_bytes, | cx, mut event | {
                if let Event::Draw = event {return (*appcx.0).draw_app(cx);}
                (*appcx.0).handle_app(cx, &mut event);
            })
        }
    };
}