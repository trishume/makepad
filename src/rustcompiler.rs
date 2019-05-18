use widget::*;
use editor::*;

use std::io::Read;
use std::process::{Command, Child, Stdio};
use std::sync::mpsc;

use serde_json::{Result};
use serde::*;

//#[derive(Clone)]
pub struct RustCompiler{
    pub view:View<ScrollBar>,
    pub bg:Quad,
    pub text:Text,
    pub item_bg:Quad,
    pub code_icon:CodeIcon,
    pub row_height:f32,
    pub path_color:Color,
    pub message_color:Color,
    pub _check_signal_id:u64,

    pub _check_child:Option<Child>,
    pub _build_child:Option<Child>,
    pub _run_child:Option<Child>,
    pub _run_when_done:bool,

    pub _rustc_build_stages:BuildStage,
    pub _rx:Option<mpsc::Receiver<std::vec::Vec<u8>>>,

    pub _thread:Option<std::thread::JoinHandle<()>>,

    pub _data:Vec<String>,
    pub _rustc_spans:Vec<RustcSpan>,
    pub _rustc_messages:Vec<RustcCompilerMessage>,
    pub _rustc_done:bool,
    pub _rustc_artifacts:Vec<RustcCompilerArtifact>,
    pub _items:Vec<RustCompilerItem>
}

const SIGNAL_RUST_CHECKER:u64 = 1;
const SIGNAL_BUILD_COMPLETE:u64 = 2;
const SIGNAL_RUN_OUTPUT:u64 = 3;

#[derive(PartialEq)]
pub enum BuildStage{
    NotRunning,
    Building,
    Complete
}

pub struct RustCompilerItem{
    hit_state:HitState,
    animator:Animator,
    span_index:usize,
    selected:bool
}

#[derive(Clone)]
pub enum RustCompilerEvent{
    SelectMessage{path:String},
    None,
}

impl Style for RustCompiler{
    fn style(cx:&mut Cx)->Self{
        Self{
            bg:Quad{
                ..Style::style(cx)
            },
            item_bg:Quad{
                ..Style::style(cx)
            },
            text:Text{
                ..Style::style(cx)
            },
            view:View{
                scroll_h:Some(ScrollBar{
                    ..Style::style(cx)
                }),
                scroll_v:Some(ScrollBar{
                    smoothing:Some(0.15),
                    ..Style::style(cx)
                }),
                ..Style::style(cx)
            },
            code_icon:CodeIcon{
                ..Style::style(cx)
            },
            path_color:color("#999"),
            message_color:color("#bbb"),
            row_height:20.0,
            _check_signal_id:0,
            _check_child:None,
            _build_child:None,
            _run_child:None,
            _run_when_done:false,
            _rustc_build_stages:BuildStage::NotRunning,
            _thread:None,
            _rx:None,
            _rustc_spans:Vec::new(),
            _rustc_messages:Vec::new(),
            _rustc_artifacts:Vec::new(),
            _rustc_done:false,
            _items:Vec::new(),
            _data:Vec::new()
        }
    }
}

impl RustCompiler{
    pub fn init(&mut self, cx:&mut Cx){
        self._check_signal_id = cx.new_signal_id();
        self.restart_rust_checker();
    }

    pub fn get_default_anim(cx:&Cx, counter:usize, marked:bool)->Anim{
        Anim::new(Play::Chain{duration:0.01}, vec![
            Track::color("bg.color", Ease::Lin, vec![(1.0,
                if marked{cx.color("bg_marked")}  else if counter&1==0{cx.color("bg_selected")}else{cx.color("bg_odd")}
            )])
        ])
    }

    pub fn get_over_anim(cx:&Cx, counter:usize, marked:bool)->Anim{
        let over_color = if marked{cx.color("bg_marked_over")} else if counter&1==0{cx.color("bg_selected_over")}else{cx.color("bg_odd_over")};
        Anim::new(Play::Cut{duration:0.02}, vec![
            Track::color("bg.color", Ease::Lin, vec![
                (0., over_color),(1., over_color)
            ])
        ])
    }

    pub fn export_messages(&self, cx:&mut Cx, text_buffers:&mut TextBuffers){
        
        for span in &self._rustc_spans{
            if span.label.is_none(){
                continue;
            }

            let text_buffer = text_buffers.from_path(cx, &span.file_name);
            let messages = &mut text_buffer.messages;
            messages.mutation_id = text_buffer.mutation_id;
            if messages.gc_id != cx.event_id{
                messages.gc_id = cx.event_id;
                messages.cursors.truncate(0);
                messages.bodies.truncate(0);
            }
            if span.byte_start == span.byte_end{
                messages.cursors.push(TextCursor{
                    head:(span.byte_start-1) as usize,
                    tail:span.byte_end as usize,
                    max:0
                });
            }
            else{
                messages.cursors.push(TextCursor{
                    head:span.byte_start as usize,
                    tail:span.byte_end as usize,
                    max:0 
                });
            }
            //println!("PROCESING MESSAGES FOR {} {} {}", span.byte_start, span.byte_end+1, path);
            text_buffer.messages.bodies.push(TextBufferMessage{
                body:span.label.clone().unwrap(),
                level:match span.level.as_ref().unwrap().as_ref(){
                    "warning"=>TextBufferMessageLevel::Warning,
                    "error"=>TextBufferMessageLevel::Error,
                    _=>TextBufferMessageLevel::Warning
                }
            });
            //}
        }
        // clear all files we missed
        for (_, text_buffer) in &mut text_buffers.storage{
            if text_buffer.messages.gc_id != cx.event_id{
                text_buffer.messages.cursors.truncate(0);
                text_buffer.messages.bodies.truncate(0);
            }
            else{
                cx.send_signal_before_draw(text_buffer.signal_id, SIGNAL_TEXTBUFFER_MESSAGE_UPDATE);
            }
        }
    }

    pub fn handle_rust_compiler(&mut self, cx:&mut Cx, event:&mut Event, text_buffers:&mut TextBuffers)->RustCompilerEvent{
        // do shit here
        if self.view.handle_scroll_bars(cx, event){
            // do zshit.
        }

        let mut item_to_select = None;

        match event{
            Event::KeyDown(ke)=>match ke.key_code{
                KeyCode::F9=>{
                    if self._rustc_build_stages == BuildStage::Complete{
                        self.run_program();
                    }
                    else{
                        self._run_when_done = true;
                        self.view.redraw_view_area(cx);
                    }
                },
                KeyCode::F8=>{ // next error
                    if self._items.len() > 0{
                        if ke.modifiers.shift{
                            let mut selected_index = None;
                            for (counter,item) in self._items.iter_mut().enumerate(){
                                if item.selected{
                                    selected_index = Some(counter);
                                }
                            }
                            if let Some(selected_index) = selected_index{
                                if selected_index > 0{
                                    item_to_select = Some(selected_index - 1);
                                }
                                else {
                                    item_to_select = Some(self._items.len() - 1);
                                }
                            }
                            else{
                                item_to_select = Some(self._items.len() - 1);
                            }
                        }
                        else{
                            let mut selected_index = None;
                            for (counter,item) in self._items.iter_mut().enumerate(){
                                if item.selected{
                                    selected_index = Some(counter);
                                }
                            }
                            if let Some(selected_index) = selected_index{
                                if selected_index + 1 < self._items.len(){
                                    item_to_select = Some(selected_index + 1);
                                }
                                else {
                                    item_to_select = Some(0);
                                }
                            }
                            else{
                                item_to_select = Some(0);
                            }
                        }
                    }
                },
                _=>()
            },
            Event::Signal(se)=>{
                if self._check_signal_id == se.signal_id{
                    match se.value{
                        SIGNAL_RUST_CHECKER=>{
                            let mut datas = Vec::new();
                            if let Some(rx) = &self._rx{
                                while let Ok(data) = rx.try_recv(){
                                    datas.push(data);
                                }
                            }
                            if datas.len() > 0{
                                self.process_compiler_messages(cx, datas);
                                self.export_messages(cx, text_buffers);
                            }
                        },
                        SIGNAL_BUILD_COMPLETE=>{
                            self._rustc_build_stages = BuildStage::Complete;
                            if self._run_when_done{
                                self.run_program();
                            }
                            self.view.redraw_view_area(cx);
                        },
                        _=>()
                    }
                }
            },
            _=>()
        }

        //let mut unmark_nodes = false;
        for (counter,item) in self._items.iter_mut().enumerate(){   
            match event.hits(cx, item.animator.area, &mut item.hit_state){
                Event::Animate(ae)=>{
                    item.animator.calc_write(cx, "bg.color", ae.time, item.animator.area);
                },
                Event::FingerDown(_fe)=>{
                    cx.set_down_mouse_cursor(MouseCursor::Hand);
                    // mark ourselves, unmark others
                    item_to_select = Some(counter);
                },
                Event::FingerUp(_fe)=>{
                },
                Event::FingerMove(_fe)=>{
                },
                Event::FingerHover(fe)=>{
                    cx.set_hover_mouse_cursor(MouseCursor::Hand);
                    match fe.hover_state{
                        HoverState::In=>{
                            item.animator.play_anim(cx, Self::get_over_anim(cx, counter, item.selected));
                        },
                        HoverState::Out=>{
                            item.animator.play_anim(cx, Self::get_default_anim(cx, counter, item.selected));
                        },
                        _=>()
                    }
                },
                _=>()
            }
        };

        if let Some(item_to_select) = item_to_select{
            
            for (counter,item) in self._items.iter_mut().enumerate(){   
                if counter != item_to_select{
                    item.selected = false;
                    item.animator.play_anim(cx, Self::get_default_anim(cx, counter, false));
                }
            };

            let item = &mut self._items[item_to_select];
            item.selected  = true;
            item.animator.play_anim(cx, Self::get_over_anim(cx, item_to_select, true));

            let span = &self._rustc_spans[item.span_index];
            // alright we clicked an item. now what. well 
            let text_buffer = text_buffers.from_path(cx, &span.file_name);
            text_buffer.messages.jump_to_offset = span.byte_start as usize;
            cx.send_signal_after_draw(text_buffer.signal_id, SIGNAL_TEXTBUFFER_JUMP_TO_OFFSET);
            return RustCompilerEvent::SelectMessage{path:span.file_name.clone()}
        }
        RustCompilerEvent::None
    }

    pub fn draw_rust_compiler(&mut self, cx:&mut Cx){
        if let Err(_) = self.view.begin_view(cx, &Layout{..Default::default()}){
            return
        }

        let mut counter = 0;
        for (index,span) in self._rustc_spans.iter().enumerate(){
            if span.label.is_none(){
                continue;
            }
            // reuse or overwrite a slot
             if counter >= self._items.len(){
                self._items.push(RustCompilerItem{
                    animator:Animator::new(Self::get_default_anim(cx, counter, false)),
                    hit_state:HitState{..Default::default()},
                    span_index:index,
                    selected:false
                });
            };
            self.item_bg.color =  self._items[counter].animator.last_color("bg.color");

            let bg_inst = self.item_bg.begin_quad(cx,&Layout{
                width:Bounds::Fill,
                height:Bounds::Fix(self.row_height),
                padding:Padding{l:2.,t:3.,b:2.,r:0.},
                ..Default::default()
            });

            if let Some(level) = &span.level{
                if level == "error"{
                    self.code_icon.draw_icon_walk(cx, CodeIconType::Error);
                }
                else{
                    self.code_icon.draw_icon_walk(cx, CodeIconType::Warning);
                }
            }

            self.text.color = self.path_color;
            self.text.draw_text(cx, &format!("{}:{} - ", span.file_name, span.line_start));
            self.text.color = self.message_color;
            self.text.draw_text(cx, &format!("{}", span.label.as_ref().unwrap()));

            let bg_area = self.item_bg.end_quad(cx, &bg_inst);
            self._items[counter].animator.update_area_refs(cx, bg_area);

            cx.turtle_new_line();
            counter += 1;
        }

        let bg_even = cx.color("bg_selected");
        let bg_odd = cx.color("bg_odd");
    
        self.item_bg.color = if counter&1 == 0{bg_even}else{bg_odd};
        let bg_inst = self.item_bg.begin_quad(cx,&Layout{
            width:Bounds::Fill,
            height:Bounds::Fix(self.row_height),
            padding:Padding{l:2.,t:3.,b:2.,r:0.},
            ..Default::default()
        });
        if self._rustc_done == true{
            self.code_icon.draw_icon_walk(cx, CodeIconType::Ok);//if any_error{CodeIconType::Error}else{CodeIconType::Warning});
            self.text.color = self.path_color;
            match self._rustc_build_stages{
                BuildStage::NotRunning=>self.text.draw_text(cx, "Done"),
                BuildStage::Building=>{
                    if self._run_when_done{
                        self.text.draw_text(cx, "Running when ready")
                    }
                    else{
                        self.text.draw_text(cx, "Building")
                    }
                },
                BuildStage::Complete=>{
                    self.text.draw_text(cx, "Press F9 to run")
                }
            };
        }
        else{
            self.code_icon.draw_icon_walk(cx, CodeIconType::Wait);
            self.text.color = self.path_color;
            self.text.draw_text(cx, &format!("Checking({})",self._rustc_artifacts.len()));
        }
        self.item_bg.end_quad(cx, &bg_inst);
        cx.turtle_new_line();
        counter += 1;

        // draw filler nodes
        
        let view_total = cx.get_turtle_bounds();   
        let rect_now =  cx.get_turtle_rect();
        let mut y = view_total.y;
        while y < rect_now.h{
            self.item_bg.color = if counter&1 == 0{bg_even}else{bg_odd};
            self.item_bg.draw_quad_walk(cx,
                Bounds::Fill,
                Bounds::Fix( (rect_now.h - y).min(self.row_height) ),
                Margin::zero()
            );
            cx.turtle_new_line();
            y += self.row_height;
            counter += 1;
        } 

        self.view.end_view(cx);
    }

    pub fn start_rust_builder(&mut self){
        if let Some(child) = &mut self._build_child{
            let _= child.kill();
        }
        if let Some(child) = &mut self._run_child{
            let _= child.kill();
        }
        // start a release build
        self._rustc_build_stages = BuildStage::Building;

        let mut _child = Command::new("cargo")
            //.args(&["build","--release","--message-format=json"])
            .args(&["build", "--release", "--message-format=json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir("./edit_repo")
            .spawn();


        let mut child = _child.unwrap();

        let mut stdout =  child.stdout.take().unwrap();
        let signal_id = self._check_signal_id;
        std::thread::spawn(move ||{
            loop{
                let mut data = vec![0; 4096];
                let n_bytes_read = stdout.read(&mut data).expect("cannot read");
                data.truncate(n_bytes_read);
                if n_bytes_read == 0{
                    Cx::send_signal(signal_id, SIGNAL_BUILD_COMPLETE);
                    return 
                }
            }
        });
        self._build_child = Some(child);
    }

     pub fn run_program(&mut self){
        self._run_when_done = false;
        if let Some(child) = &mut self._run_child{
            let _= child.kill();
        }

        let mut _child = Command::new("cargo")
            .args(&["run", "--release"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir("./edit_repo")
            .spawn();

        let mut child = _child.unwrap();

        let mut stdout =  child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        let signal_id = self._check_signal_id;
        let thread = std::thread::spawn(move ||{
            loop{
                let mut data = vec![0; 4096];
                let n_bytes_read = stdout.read(&mut data).expect("cannot read");
                data.truncate(n_bytes_read);
                let _ = tx.send(data);
                Cx::send_signal(signal_id, SIGNAL_RUN_OUTPUT);
                if n_bytes_read == 0{
                    return 
                }
            }
        });
        self._rx = Some(rx);
        self._thread = Some(thread);
        self._run_child = Some(child);
    }

    pub fn restart_rust_checker(&mut self){
        self._data.truncate(0);
        self._rustc_messages.truncate(0);
        self._rustc_spans.truncate(0);
        self._rustc_artifacts.truncate(0);
        self._rustc_done = false;
        self._rustc_build_stages = BuildStage::NotRunning;
        self._items.truncate(0);
        self._data.push(String::new());

        if let Some(child) = &mut self._check_child{
            let _= child.kill();
        }

         if let Some(child) = &mut self._build_child{
            let _= child.kill();
        }

        let mut _child = Command::new("cargo")
            //.args(&["build","--release","--message-format=json"])
            .args(&["check","--message-format=json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir("./edit_repo")
            .spawn();
        
        if let Err(_) = _child{
            self._rustc_spans.push(RustcSpan{
                level:Some("error".to_string()),
                label:Some("A Rust compiler error, only works native".to_string()),
                ..Default::default()
            });
            self._rustc_spans.push(RustcSpan{
                level:Some("warning".to_string()),
                label:Some("A Rust compiler warning, only works native".to_string()),
                ..Default::default()
            });
            return;
        }
        let mut child = _child.unwrap();

        //let mut stderr =  child.stderr.take().unwrap();
        let mut stdout =  child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        let signal_id = self._check_signal_id;
        let thread = std::thread::spawn(move ||{
            loop{
                let mut data = vec![0; 4096];
                let n_bytes_read = stdout.read(&mut data).expect("cannot read");
                data.truncate(n_bytes_read);
                let _ = tx.send(data);
                Cx::send_signal(signal_id, SIGNAL_RUST_CHECKER);
                if n_bytes_read == 0{
                    return 
                }
            }
        });
        self._rx = Some(rx);
        self._thread = Some(thread);
        self._check_child = Some(child);
    }

     pub fn process_compiler_messages(&mut self, cx:&mut Cx, datas:Vec<Vec<u8>>){
        for data in datas{
            if data.len() == 0{ // last event
                self._rustc_done = true;
                // the check is done, do we have any errors? ifnot start a release build
                let mut has_errors = false;
                for span in &self._rustc_spans{
                    if let Some(level) = &span.level{
                        if level == "error"{
                            has_errors = true;
                            break;
                        }
                    }
                }
                if !has_errors{ // start release build
                    self.start_rust_builder();
                }
                self.view.redraw_view_area(cx);
            }
            else {
                for ch in data{
                    if ch == '\n' as u8{
                        // parse it
                        let line = self._data.last_mut().unwrap();
                        // parse the line
                        if line.contains("\"reason\":\"compiler-artifact\""){
                            let parsed:Result<RustcCompilerArtifact> = serde_json::from_str(line); 
                            match parsed{
                                Err(err)=>println!("JSON PARSE ERROR {:?} {}", err, line),
                                Ok(parsed)=>{
                                    self._rustc_artifacts.push(parsed);
                                }
                            }
                            self.view.redraw_view_area(cx);
                        }
                        else if line.contains("\"reason\":\"compiler-message\""){
                            let parsed:Result<RustcCompilerMessage> = serde_json::from_str(line); 
                            match parsed{
                                Err(err)=>println!("JSON PARSE ERROR {:?} {}", err, line),
                                Ok(parsed)=>{
                                    let spans = &parsed.message.spans;
                                    if spans.len() > 0{
                                        for i in 0..spans.len(){
                                            let mut span = spans[i].clone();
                                            if !span.is_primary{
                                                continue
                                            }
                                            if span.label.is_none(){
                                                span.label = Some(parsed.message.message.clone());
                                            }
                                            //span.file_name = format!("/{}",span.file_name);
                                            span.level = Some(parsed.message.level.clone());
                                            self._rustc_spans.push(span);
                                        }
                                    }
                                    self._rustc_messages.push(parsed);
                                }
                            }
                            self.view.redraw_view_area(cx);
                        }
                        self._data.push(String::new());
                    }
                    else{
                        self._data.last_mut().unwrap().push(ch as char);
                    }
                }
            }
        }
    }
}

#[derive(Clone, Deserialize,  Default)]
pub struct RustcTarget{
    kind:Vec<String>,
    crate_types:Vec<String>,
    name:String,
    src_path:String,
    edition:String
}

#[derive(Clone, Deserialize,  Default)]
pub struct RustcText{
    text:String,
    highlight_start:u32,
    highlight_end:u32
}

#[derive(Clone, Deserialize,  Default)]
pub struct RustcSpan{
    file_name:String,
    byte_start:u32,
    byte_end:u32,
    line_start:u32,
    line_end:u32,
    column_start:u32,
    column_end:u32,
    is_primary:bool,
    text:Vec<RustcText>,
    label:Option<String>,
    suggested_replacement:Option<String>,
    sugggested_applicability:Option<String>,
    expansion:Option<Box<RustcExpansion>>,
    level:Option<String>
}

#[derive(Clone, Deserialize,  Default)]
pub struct RustcExpansion{
    span:Option<RustcSpan>,
    macro_decl_name:String,
    def_site_span:Option<RustcSpan>
}

#[derive(Clone, Deserialize,  Default)] 
pub struct RustcCode{
    code:String,
    explanation:Option<String>
}

#[derive(Clone, Deserialize,  Default)]
pub struct RustcMessage{
    message:String,
    code:Option<RustcCode>,
    level:String,
    spans:Vec<RustcSpan>,
    children:Vec<RustcMessage>,
    rendered:Option<String>
}

#[derive(Clone, Deserialize,  Default)]
pub struct RustcProfile{
    opt_level:String,
    debuginfo:Option<u32>,
    debug_assertions:bool,
    overflow_checks:bool,
    test:bool
}

#[derive(Clone, Deserialize,  Default)]
pub struct RustcCompilerMessage{
    reason:String,
    package_id:String,
    target:RustcTarget,
    message:RustcMessage
}

#[derive(Clone, Deserialize,  Default)]
pub struct RustcCompilerArtifact{
    reason:String,
    package_id:String,
    target:RustcTarget,
    profile:RustcProfile,
    features:Vec<String>,
    filenames:Vec<String>,
    executable:Option<String>,
    fresh:bool
}