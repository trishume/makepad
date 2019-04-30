use widgets::*;
use crate::textbuffer::*;

#[derive(Clone)]
pub struct CodeEditor{
    pub view:View<ScrollBar>,
    pub bg_layout:Layout,
    pub bg: Quad,
    pub cursor: Quad,
    pub marker: Quad,
    pub tab:Quad,
    pub text: Text,
    pub cursors:CursorSet,
    pub _hit_state:HitState,
    pub _bg_area:Area,
    pub _text_inst:Option<AlignedInstance>,
    pub _text_area:Area,
    pub _scroll_pos:Vec2,
    pub _last_finger_move:Option<Vec2>,
    pub _line_geometry:Vec<LineGeom>,
    pub _visibility_margin:Margin,
    pub _select_scroll:Option<SelectScroll>,
    
    pub _monospace_size:Vec2,
    pub _instance_count:usize,
    pub _first_on_line:bool,
    pub _draw_cursor:DrawCursor
}

#[derive(Clone, Default)]
pub struct LineGeom{
    walk:Vec2,
    font_size:f32
}

#[derive(Clone, Default)]
pub struct SelectScroll{
    pub margin:Margin,
    pub delta:Vec2,
    pub abs:Vec2
}

impl ElementLife for CodeEditor{
    fn construct(&mut self, _cx:&mut Cx){}
    fn destruct(&mut self, _cx:&mut Cx){}
}

impl Style for CodeEditor{
    fn style(cx:&mut Cx)->Self{
        let tab_sh = Self::def_tab_shader(cx);
        let marker_sh = Self::def_marker_shader(cx);
        let cursor_sh = Self::def_cursor_shader(cx);
        let code_editor = Self{
            cursors:CursorSet::new(),
            tab:Quad{
                color:color("#5"),
                shader_id:cx.add_shader(tab_sh, "Editor.tab"),
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
            bg:Quad{
                color:color256(30,30,30),
                do_scroll:false,
                ..Style::style(cx)
            },
            marker:Quad{
                color:color256(42,78,117),
                shader_id:cx.add_shader(marker_sh, "Editor.marker"),
                ..Style::style(cx)
            }, 
            cursor:Quad{
                color:color256(136,136,136),
                shader_id:cx.add_shader(cursor_sh, "Editor.cursor"),
                ..Style::style(cx)
            },
            bg_layout:Layout{
                width:Bounds::Fill,
                height:Bounds::Fill,
                margin:Margin::all(0.),
                padding:Padding{l:4.0,t:4.0,r:4.0,b:4.0},
                ..Default::default()
            },
            text:Text{
                font_id:cx.load_font(&cx.font("mono_font")),
                font_size:11.0,
                brightness:1.05,
                line_spacing:1.4,
                wrapping:Wrapping::Line,
                ..Style::style(cx)
            },
            _hit_state:HitState{no_scrolling:true, ..Default::default()},
            _monospace_size:Vec2::zero(),
            _last_finger_move:None,
            _first_on_line:true,
            _scroll_pos:Vec2::zero(),
            _visibility_margin:Margin::zero(),
            _line_geometry:Vec::new(),
            _bg_area:Area::Empty,
            _text_inst:None,
            _text_area:Area::Empty,
            _instance_count:0,
            _select_scroll:None,
            _draw_cursor:DrawCursor::new()
        };
        //tab.animator.default = tab.anim_default(cx);
        code_editor
    }
}

#[derive(Clone, PartialEq)]
pub enum CodeEditorEvent{
    None,
    Change
}

impl CodeEditor{

    pub fn def_tab_shader(cx:&mut Cx)->Shader{
        let mut sh = Quad::def_quad_shader(cx);
        sh.add_ast(shader_ast!({
            fn pixel()->vec4{
                df_viewport(pos * vec2(w, h));
                df_move_to(1.,-1.);
                df_line_to(1.,h+1.);
                return df_stroke(color, 0.8);
            }
        }));
        sh
    }

    pub fn def_cursor_shader(cx:&mut Cx)->Shader{
        let mut sh = Quad::def_quad_shader(cx);
        sh.add_ast(shader_ast!({
            fn pixel()->vec4{
                return vec4(color.rgb*color.a,color.a)
            }
        }));
        sh
    }

    pub fn def_marker_shader(cx:&mut Cx)->Shader{
        let mut sh = Quad::def_quad_shader(cx);
        sh.add_ast(shader_ast!({
            let prev_x:float<Instance>;
            let prev_w:float<Instance>;
            let next_x:float<Instance>;
            let next_w:float<Instance>;
            const gloopiness:float = 8.;
            const border_radius:float = 2.;

            fn vertex()->vec4{ // custom vertex shader because we widen the draweable area a bit for the gloopiness
                let shift:vec2 = -draw_list_scroll * draw_list_do_scroll;
                let clipped:vec2 = clamp(
                    geom*vec2(w+16., h) + vec2(x, y) + shift - vec2(8.,0.),
                    draw_list_clip.xy,
                    draw_list_clip.zw
                );
                pos = (clipped - shift - vec2(x,y)) / vec2(w, h);
                return vec4(clipped,0.,1.) * camera_projection;
            }

            fn pixel()->vec4{
                df_viewport(pos * vec2(w, h));
                df_box(0., 0., w, h, border_radius);
                if prev_w > 0.{
                    df_box(prev_x, -h, prev_w, h, border_radius);
                    df_gloop(gloopiness);
                }
                if next_w > 0.{
                    df_box(next_x, h, next_w, h, border_radius);
                    df_gloop(gloopiness);
                }
                return df_fill(color);
            }
        }));
        sh
    }

    pub fn handle_code_editor(&mut self, cx:&mut Cx, event:&mut Event, text_buffer:&mut TextBuffer)->CodeEditorEvent{
        match self.view.handle_scroll_bars(cx, event){
            (_,ScrollBarEvent::Scroll{..}) | (ScrollBarEvent::Scroll{..},_)=>{
                if let Some(last_finger_move) = self._last_finger_move{
                    let offset = self.text.find_closest_offset(cx, &self._text_area, last_finger_move);
                    self.cursors.update_cursor_drag(offset, text_buffer);
                }
                // the editor actually redraws on scroll, its because we don't actually
                // generate the entire file as GPU text-buffer just the visible area
                // in JS this wasn't possible performantly but in Rust its a breeze.
                self.view.redraw_view_area(cx);
            },
            _=>()
        }
        match event.hits(cx, self._bg_area, &mut self._hit_state){

            Event::Animate(_ae)=>{
            },
            Event::FingerDown(fe)=>{
                cx.set_down_mouse_cursor(MouseCursor::Text);
                // give us the focus
                cx.set_key_focus(self._bg_area);
                let offset = self.text.find_closest_offset(cx, &self._text_area, fe.abs);
                self.cursors.begin_cursor_drag(fe.modifier.logo, offset, text_buffer);
                self.view.redraw_view_area(cx);
                self._last_finger_move = Some(fe.abs);
            },
            Event::FingerHover(_fe)=>{
                cx.set_hover_mouse_cursor(MouseCursor::Text);
            },
            Event::FingerUp(_fe)=>{
                self.cursors.end_cursor_drag(text_buffer);
                self._select_scroll = None;
                self._last_finger_move = None;
            },
            Event::FingerMove(fe)=>{
                let offset = self.text.find_closest_offset(cx, &self._text_area, fe.abs);
                self.cursors.update_cursor_drag(offset, text_buffer);

                self._last_finger_move = Some(fe.abs);
                // determine selection drag scroll dynamics
                let pow_scale = 0.1;
                let pow_fac = 3.;
                let max_speed = 40.;
                let pad_scroll = 20.;
                let rect = Rect{
                    x:fe.rect.x+pad_scroll,
                    y:fe.rect.y+pad_scroll,
                    w:fe.rect.w-2.*pad_scroll,
                    h:fe.rect.h-2.*pad_scroll,
                };
                let delta = Vec2{
                    x:if fe.abs.x < rect.x{
                        -((rect.x - fe.abs.x) * pow_scale).powf(pow_fac).min(max_speed)
                    }
                    else if fe.abs.x > rect.x + rect.w{
                        ((fe.abs.x - (rect.x + rect.w)) * pow_scale).powf(pow_fac).min(max_speed)
                    }
                    else{
                        0.
                    },
                    y:if fe.abs.y < rect.y{
                        -((rect.y - fe.abs.y) * pow_scale).powf(pow_fac).min(max_speed)
                    }
                    else if fe.abs.y > rect.y + rect.h{
                        ((fe.abs.y - (rect.y + rect.h)) * pow_scale).powf(pow_fac).min(max_speed)
                    }
                    else{
                        0.
                    }
                };
                let last_scroll_none = self._select_scroll.is_none();
                if delta.x !=0. || delta.y != 0.{
                   self._select_scroll = Some(SelectScroll{
                       abs:fe.abs,
                       delta:delta,
                       margin:Margin{
                            l:(-delta.x).max(0.),
                            t:(-delta.y).max(0.),
                            r:delta.x.max(0.),
                            b:delta.y.max(0.)
                        }
                   })
                }
                else{
                    self._select_scroll = None;
                }
                if last_scroll_none{
                    self.view.redraw_view_area(cx);
                }
            },
            Event::KeyDown(ke)=>{
                let cursor_moved = match ke.key_code{
                    KeyCode::ArrowUp=>{
                        self.cursors.move_up(1, ke.modifier.shift, text_buffer);
                        true
                    },
                    KeyCode::ArrowDown=>{
                        self.cursors.move_down(1, ke.modifier.shift, text_buffer);
                        true
                    },
                    KeyCode::ArrowLeft=>{
                        self.cursors.move_left(1, ke.modifier.shift, text_buffer);
                        true
                    },
                    KeyCode::ArrowRight=>{
                        self.cursors.move_right(1, ke.modifier.shift, text_buffer);
                        true
                    },
                    KeyCode::PageUp=>{
                        self.cursors.move_up(self._line_geometry.len().min(20), ke.modifier.shift, text_buffer);
                        true
                    },
                    KeyCode::PageDown=>{
                        self.cursors.move_down(self._line_geometry.len().min(20), ke.modifier.shift, text_buffer);
                        true
                    },
                    KeyCode::Home=>{
                        self.cursors.move_home(ke.modifier.shift, text_buffer);
                        true
                    },
                    KeyCode::End=>{
                        self.cursors.move_end(ke.modifier.shift, text_buffer);
                        true
                    },
                    _=>false
                };
                if cursor_moved{
                    self.scroll_last_cursor_visible(cx, text_buffer);
                    self.view.redraw_view_area(cx);
                }
            },
            Event::TextInput(te)=>{
                //println!("TextInput {:?}", te);
            }
            _=>()
        };
        CodeEditorEvent::None
   }

    pub fn begin_code_editor(&mut self, cx:&mut Cx, text_buffer:&TextBuffer)->bool{
        // pull the bg color from our animation system, uses 'default' value otherwise
        // self.bg.color = self.animator.last_vec4("bg.color");
        // push the 2 vars we added to bg shader
        //self.text.color = self.animator.last_vec4("text.color");
        self.view.begin_view(cx, &Layout{..Default::default()});
        //   return false
        //}
        if text_buffer.load_id != 0{
            let bg_inst = self.bg.begin_quad(cx, &Layout{
                align:Align::left_top(),
                ..self.bg_layout.clone()
            });
            self.text.color = color("#666");
            self.text.draw_text(cx, "...");
            self.bg.end_quad(cx, &bg_inst);
            self._bg_area = bg_inst.into_area();
            self.view.end_view(cx);
            return false
        }
        else{

            let bg_inst = self.bg.draw_quad(cx, Rect{x:0.,y:0., w:cx.width_total(false), h:cx.height_total(false)});
            let bg_area = bg_inst.into_area();
            cx.update_area_refs(self._bg_area, bg_area);
            self._bg_area = bg_area;
            // makers before text
            cx.new_instance_layer(self.marker.shader_id, 0);

            self._text_inst = Some(self.text.begin_text(cx));
            self._instance_count = 0;

            self._scroll_pos = self.view.get_scroll_pos(cx);

            self._visibility_margin = if let Some(select_scroll) = &self._select_scroll{
                select_scroll.margin
            }
            else{
                Margin::zero()
            };

            self._monospace_size = self.text.get_monospace_size(cx, None);
            self._line_geometry.truncate(0);
            self._draw_cursor = DrawCursor::new();
            self._first_on_line = true;
            // prime the next cursor
            self._draw_cursor.set_next(&self.cursors.set);
            // cursor after text
            cx.new_instance_layer(self.cursor.shader_id, 0);
            
            return true
        }
    }
    
    pub fn end_code_editor(&mut self, cx:&mut Cx, text_buffer:&TextBuffer){
        // lets insert an empty newline at the bottom so its nicer to scroll
        cx.turtle_new_line();
        cx.walk_turtle(Bounds::Fix(0.0),  Bounds::Fix(self._monospace_size.y),  Margin::zero(), None);
        
        self.text.end_text(cx, self._text_inst.as_ref().unwrap());
        // lets draw cursors and selection rects.
        let draw_cursor = &self._draw_cursor;
        let pos = cx.turtle_origin();
        cx.new_instance_layer(self.cursor.shader_id, 0);

        // draw the cursors    
        for rc in &draw_cursor.cursors{
           self.cursor.draw_quad(cx, Rect{x:rc.x - pos.x, y:rc.y - pos.y, w:rc.w, h:rc.h});
        }
        
        self._text_area = self._text_inst.take().unwrap().inst.into_area();

        // draw selections
        let sel = &draw_cursor.selections;
        for i in 0..sel.len(){
            let cur = &sel[i];
            let mk_inst = self.marker.draw_quad(cx, Rect{x:cur.rc.x - pos.x, y:cur.rc.y - pos.y, w:cur.rc.w, h:cur.rc.h});
            // do we have a prev?
            if i > 0 && sel[i-1].index == cur.index{
                let p_rc = &sel[i-1].rc;
                mk_inst.push_vec2(cx, Vec2{x:p_rc.x - cur.rc.x, y:p_rc.w}); // prev_x, prev_w
            }
            else{
                mk_inst.push_vec2(cx, Vec2{x:0., y:-1.}); // prev_x, prev_w
            }
            // do we have a next
            if i < sel.len() - 1 && sel[i+1].index == cur.index{
                let n_rc = &sel[i+1].rc;
                mk_inst.push_vec2(cx, Vec2{x:n_rc.x - cur.rc.x, y:n_rc.w}); // prev_x, prev_w
            }
            else{
                mk_inst.push_vec2(cx, Vec2{x:0., y:-1.}); // prev_x, prev_w
            }
        }

        // do select scrolling
        if let Some(select_scroll) = &self._select_scroll{
            let offset = self.text.find_closest_offset(cx, &self._text_area, select_scroll.abs);
            self.cursors.update_cursor_drag(offset, text_buffer);
            if self.view.set_scroll_pos(cx, Vec2{
                x:self._scroll_pos.x + select_scroll.delta.x,
                y:self._scroll_pos.y + select_scroll.delta.y
            }){
                self.view.redraw_view_area(cx);
            }
            else{
                self._select_scroll = None;
            }
        }

        self.view.end_view(cx);
    }

    pub fn draw_tab_lines(&mut self, cx:&mut Cx, tabs:usize){
        let walk = cx.get_turtle_walk();
        let tab_width = self._monospace_size.x*4.;
        if cx.visible_in_turtle(
            Rect{x:walk.x, y:walk.y, w:tab_width * tabs as f32, h:self._monospace_size.y}, 
            self._visibility_margin, 
            self._scroll_pos,
        ){
            for _i in 0..tabs{
                self.tab.draw_quad_walk(cx, Bounds::Fix(tab_width), Bounds::Fix(self._monospace_size.y), Margin::zero());
            }   
            cx.set_turtle_walk(walk);
        }
    }

    // set it once per line otherwise the LineGeom stuff isn't really working out.
    pub fn set_font_size(&mut self, cx:&Cx, font_size:f32){
        self.text.font_size = font_size;
        self._monospace_size = self.text.get_monospace_size(cx, None);
    }

    pub fn new_line(&mut self, cx:&mut Cx){
        // line geometry is used for scrolling look up of cursors
        self._line_geometry.push(
            LineGeom{
                walk:cx.get_rel_turtle_walk(),
                font_size:self.text.font_size
            }
        );
        // add a bit of room to the right
        cx.walk_turtle(
            Bounds::Fix(self._monospace_size.x * 3.), 
            Bounds::Fix(self._monospace_size.y), 
            Margin::zero(),
            None
        );
        cx.turtle_new_line();
        self._first_on_line = true;
        let mut draw_cursor = &mut self._draw_cursor;
        if !draw_cursor.first{ // we have some selection data to emit
           draw_cursor.emit_selection(true);
           draw_cursor.first = true;
        }
    }

    pub fn draw_text(&mut self, cx:&mut Cx, chunk:&Vec<char>, end_offset:usize, color:Color){
        if chunk.len()>0{
            let geom = cx.walk_turtle(
                Bounds::Fix(self._monospace_size.x * (chunk.len() as f32)), 
                Bounds::Fix(self._monospace_size.y), 
                Margin::zero(),
                None
            );
            
            // lets check if the geom is visible
            if cx.visible_in_turtle(geom, self._visibility_margin, self._scroll_pos){

                if self._first_on_line{
                    self._first_on_line = false;
                }

                self.text.color = color;
                // we need to find the next cursor point we need to do something at
                let cursors = &self.cursors.set;
                let draw_cursor = &mut self._draw_cursor;
                let height = self._monospace_size.y;

                self.text.add_text(cx, geom.x, geom.y, end_offset - chunk.len() - 1, self._text_inst.as_mut().unwrap(), &chunk, |unicode, offset, x, w|{
                    // check if we need to skip cursors
                    while offset > draw_cursor.end{ // jump to next cursor
                        if !draw_cursor.set_next(cursors){ // cant go further
                            return 0.0
                        }
                    }
                    
                    // in current cursor range, update values
                    if offset >= draw_cursor.start && offset <= draw_cursor.end{
                        draw_cursor.process_geom(offset, x, geom.y, w, height);
                        if offset == draw_cursor.end{
                            draw_cursor.emit_selection(false);
                        }
                        if unicode == 10{
                            return 0.0
                        }
                        else if unicode == 32 && offset < draw_cursor.end{
                            return 2.0
                        }
                    }
                    return 0.0
                });
            }

            self._instance_count += chunk.len();
        }
    }

    fn scroll_last_cursor_visible(&mut self, cx:&mut Cx, text_buffer:&TextBuffer){
        // so we have to compute (approximately) the rect of our cursor
        if self.cursors.last_cursor >= self.cursors.set.len(){
            panic!("LAST CURSOR INVALID");
        }
        let offset = self.cursors.set[self.cursors.last_cursor].head;
        let (row, col) = text_buffer.offset_to_row_col(offset);
        // alright now lets query the line geometry
        if row < self._line_geometry.len(){
            let geom = &self._line_geometry[row];
            let mono_size = self.text.get_monospace_size(cx, Some(geom.font_size));
            let rect = Rect{
                x:(col as f32) * mono_size.x,
                y:geom.walk.y - mono_size.y * 1.,
                w:mono_size.x * 4.,
                h:mono_size.y * 3.
            };
            // scroll this cursor into view
            self.view.scroll_into_view(cx, rect);
        }
    }

/*
    pub fn cursors_replace_text(&mut self, text:&str, text_buffer:&mut TextBuffer){

    }*/
}

#[derive(Clone)]
pub struct CursorSet{
    set:Vec<Cursor>,
    last_cursor:usize
}

impl CursorSet{
    fn new()->CursorSet{
        CursorSet{
            set:vec![Cursor{head:0,tail:0,max:0}],
            last_cursor:0
        }
    }

    pub fn fuse_adjacent(&mut self, text_buffer:&TextBuffer){
        let mut index = 0;
        loop{
            if self.set.len() < 2 || index >= self.set.len() - 1{ // no more pairs
                return
            }
            // get the pair data
            let (my_start,my_end) = self.set[index].order();
            let (next_start,next_end) = self.set[index+1].order();
            if my_end >= next_start{ // fuse them together
                // check if we are mergin down or up
                if my_end < next_end{ // otherwise just remove the next
                    if self.set[index].tail>self.set[index].head{ // down
                        self.set[index].head = my_start;
                        self.set[index].tail = next_end;
                    }
                    else{ // up
                    self.set[index].head = next_end;
                    self.set[index].tail = my_start;
                    }
                    self.set[index].calc_max(text_buffer);
                    // remove the next item
                }
                if self.last_cursor > index{
                    self.last_cursor -= 1;
                }
                self.set.remove(index + 1);
            }
            index += 1;
        }
    }

    pub fn remove_collision(&mut self, offset:usize)->usize{
        // remove any cursor that intersects us
        let mut index = 0;
        loop{
            if index >= self.set.len(){
                return index
            }
            let (start,end) = self.set[index].order();
            if offset < start{
                return index
            }
            if offset >= start && offset <=end{
                if self.last_cursor > index{ // we remove a cursor before the last_cursor
                    self.last_cursor = self.last_cursor - 1;
                    self.set.remove(index);
                }
                else if self.last_cursor != index{ // it something after it so it doesnt matter
                    self.set.remove(index);
                }
                else{ // we are the last_cursor
                    index += 1;
                }
            }
            else{
                index += 1;
            }
        }
    }

    // puts the head down
    pub fn begin_cursor_drag(&mut self, add:bool, offset:usize, text_buffer:&TextBuffer){
        if !add{
            self.set.truncate(0);
        }

        let index = self.remove_collision(offset);
        
        self.set.insert(index, Cursor{
            head:offset,
            tail:offset,
            max:0
        });
        self.last_cursor = index;
        self.set[index].calc_max(text_buffer);
    }

    pub fn update_cursor_drag(&mut self, offset:usize, text_buffer:&TextBuffer){

        // remove any cursor that intersects us
        self.remove_collision(offset);

        self.set[self.last_cursor].head = offset;
        self.set[self.last_cursor].calc_max(text_buffer);
    }

    pub fn end_cursor_drag(&mut self, _text_buffer:&TextBuffer){
    }

    pub fn move_home(&mut self,only_head:bool, text_buffer:&TextBuffer){
        for cursor in &mut self.set{
            cursor.move_home(text_buffer);
            if !only_head{cursor.tail = cursor.head}
        }
        self.fuse_adjacent(text_buffer)
    }

    pub fn move_end(&mut self,only_head:bool, text_buffer:&TextBuffer){
        for cursor in &mut self.set{
            cursor.move_end(text_buffer);
            if !only_head{cursor.tail = cursor.head}
        }
        self.fuse_adjacent(text_buffer)
    }

    pub fn move_up(&mut self, line_count:usize, only_head:bool, text_buffer:&TextBuffer){
        for cursor in &mut self.set{
            cursor.move_up(line_count, text_buffer);
            if !only_head{cursor.tail = cursor.head}
        }
        self.fuse_adjacent(text_buffer)
    }

    pub fn move_down(&mut self,line_count:usize, only_head:bool, text_buffer:&TextBuffer){
        for cursor in &mut self.set{
            cursor.move_down(line_count, text_buffer);
            if !only_head{cursor.tail = cursor.head}
        }
        self.fuse_adjacent(text_buffer)
    }

    pub fn move_left(&mut self, char_count:usize, only_head:bool, text_buffer:&TextBuffer){
        for cursor in &mut self.set{
            cursor.move_left(char_count, text_buffer);
            if !only_head{cursor.tail = cursor.head}
        }
        self.fuse_adjacent(text_buffer)
    }

    pub fn move_right(&mut self,char_count:usize, only_head:bool, text_buffer:&TextBuffer){
        for cursor in &mut self.set{
            cursor.move_right(char_count, text_buffer);
            if !only_head{cursor.tail = cursor.head}
        }
        self.fuse_adjacent(text_buffer)
    }
}

#[derive(Clone)]
pub struct Cursor{
    head:usize,
    tail:usize,
    max:usize
}

impl Cursor{
    pub fn has_selection(&self)->bool{
        self.head != self.tail
    }

    pub fn order(&self)->(usize,usize){
        if self.head > self.tail{
            (self.tail,self.head)
        }
        else{
            (self.head,self.tail)
        }
    }

    pub fn calc_max(&mut self, text_buffer:&TextBuffer){
        let (_row,col) = text_buffer.offset_to_row_col(self.head);
        self.max = col;
    }

    pub fn move_home(&mut self, text_buffer:&TextBuffer){
        let (row, _col) = text_buffer.offset_to_row_col(self.head);

        // alright lets walk the line from the left till its no longer 9 or 32
        for (pos,ch) in text_buffer.lines[row].iter().enumerate(){
            if *ch != '\t' && *ch != ' '{
                self.head = text_buffer.row_col_to_offset(row, pos);
                self.calc_max(text_buffer);
                return
            }
        }
    }

    pub fn move_end(&mut self, text_buffer:&TextBuffer){
        let (row, _col) = text_buffer.offset_to_row_col(self.head);
        // alright lets walk the line from the left till its no longer 9 or 32
        self.head = text_buffer.row_col_to_offset(row, text_buffer.lines[row].len());
        self.calc_max(text_buffer);
    }

    pub fn move_left(&mut self, char_count:usize,  text_buffer:&TextBuffer){
        if self.head >= char_count{
            self.head -= char_count;
        }
        else{
            self.head = 0;
        }
        self.calc_max(text_buffer);
    }

    pub fn move_right(&mut self, char_count:usize, text_buffer:&TextBuffer){
        if self.head + char_count < text_buffer.get_char_count() - 1{
            self.head += char_count;
        }
        else{
            self.head = text_buffer.get_char_count() - 1;
        }
        self.calc_max(text_buffer);
    }

    pub fn move_up(&mut self, line_count:usize, text_buffer:&TextBuffer){
        let (row,_col) = text_buffer.offset_to_row_col(self.head);
        if row >= line_count {
            self.head = text_buffer.row_col_to_offset(row - line_count, self.max);
        }
        else{
            self.head = 0;
        }
    }
    
    pub fn move_down(&mut self, line_count:usize, text_buffer:&TextBuffer){
        let (row,_col) = text_buffer.offset_to_row_col(self.head);
        
        if row + line_count < text_buffer.get_line_count() - 1{
            
            self.head = text_buffer.row_col_to_offset(row + line_count, self.max);
        }
        else{
            self.head = text_buffer.get_char_count() - 1;
        }
    }
}


#[derive(Clone)]
pub struct DrawSel{
    index:usize,
    rc:Rect,
}

#[derive(Clone)]
pub struct DrawCursor{
    pub head:usize,
    pub start:usize,
    pub end:usize,
    pub next_index:usize,
    pub left_top:Vec2,
    pub right_bottom:Vec2,
    pub last_w:f32,
    pub first:bool,
    pub empty:bool,
    pub cursors:Vec<Rect>,
    pub selections:Vec<DrawSel>
}

impl DrawCursor{
    pub fn new()->DrawCursor{
        DrawCursor{
            start:0,
            end:0,
            head:0,
            first:true,
            empty:true,
            next_index:0,
            left_top:Vec2::zero(),
            right_bottom:Vec2::zero(),
            last_w:0.0,
            cursors:Vec::new(),
            selections:Vec::new(),
        }
    }

    pub fn set_next(&mut self, cursors:&Vec<Cursor>)->bool{
        if self.next_index < cursors.len(){
            self.emit_selection(false);
            let cursor = &cursors[self.next_index];
            let (start,end) = cursor.order();
            self.start = start;
            self.end = end;
            self.head = cursor.head;
            self.next_index += 1;
            self.last_w = 0.0;
            self.first = true;
            self.empty = true;
            true
        }
        else{
            false
        }
    }

    pub fn emit_cursor(&mut self, x:f32, y:f32, h:f32){
        self.cursors.push(Rect{
            x:x,
            y:y,
            w:1.5,
            h:h
        })
    }

    pub fn emit_selection(&mut self, on_new_line:bool){
        if !self.first{
            self.first = true;
            if !self.empty || on_new_line{
                self.selections.push(DrawSel{
                    index:self.next_index - 1,
                    rc:Rect{
                        x:self.left_top.x,
                        y:self.left_top.y,
                        w:(self.right_bottom.x - self.left_top.x) + if on_new_line{self.last_w} else {0.0},
                        h:self.right_bottom.y - self.left_top.y
                    }
                })
            }
        }
    }

    pub fn process_geom(&mut self, offset:usize, x:f32, y:f32, w:f32, h:f32){
        if offset == self.head{ // emit a cursor
            self.emit_cursor(x, y, h);
        }
        if self.first{ // store left top of rect
            self.first = false;
            self.left_top.x = x;
            self.left_top.y = y;
            self.empty = true;
        }
        else{
            self.empty = false;
        }
        // current right/bottom
        self.last_w = w;
        self.right_bottom.x = x;
        self.right_bottom.y = y + h;
    }
}