use shui::*;

#[derive(Clone, Element)]
pub struct Button{
    pub view:View,
    pub bg_area:Area,
    pub layout:Layout,
    pub bg: Quad,
    pub bg_layout:Layout,
    pub text: Text,
    pub anim:Animation<ButtonState>,
    pub label:String,
    pub event:ButtonEvent
}

#[derive(Clone, PartialEq)]
pub enum ButtonState{
    Default,
    Over,
    Down
}

impl Style for Button{
    fn style(cx:&mut Cx)->Self{
        let bg_sh = Self::def_bg_shader(cx);
        Self{
            bg_layout:Layout{
                align:Align::center(),
                w:Computed,
                h:Computed,
                margin:Margin::i32(1),
                ..Layout::paddedf(6.0,14.0,6.0,14.0)
            },

            view:View::new(),
            bg_area:Area::Empty,
            layout:Layout{
                w:Computed,
                h:Computed,
                ..Layout::new()
            },
            label:"OK".to_string(),
            anim:Animation::new(
                ButtonState::Default,
                vec![
                    AnimState::new(
                        ButtonState::Default,
                        AnimMode::Chain{duration:0.1}, 
                        vec![
                            AnimTrack::to_vec4("bg.color",cx.style.bg_normal),
                            AnimTrack::to_float("bg.glow_size",0.0),
                            AnimTrack::to_vec4("bg.border_color",cx.style.text_lo),
                            AnimTrack::to_vec4("text.color",cx.style.text_med),
                            AnimTrack::to_vec4("icon.color",cx.style.text_med),
                        ]
                    ),
                    AnimState::new(
                        ButtonState::Over,
                        AnimMode::Chain{duration:0.05}, 
                        vec![
                            AnimTrack::to_vec4("bg.color", cx.style.bg_top),
                            AnimTrack::to_vec4("bg.border_color", color("white")),
                            AnimTrack::to_float("bg.glow_size", 1.0)
                        ]
                    ),
                    AnimState::new(
                        ButtonState::Down,
                        AnimMode::Cut{duration:0.2}, 
                        vec![
                            AnimTrack::vec4("bg.border_color", vec![
                                (0.0, color("white")),
                                (1.0, color("white"))
                            ]),
                            AnimTrack::vec4("bg.color", vec![
                                (0.0, color("#f")),
                                (1.0, color("#6"))
                            ]),
                            AnimTrack::float("bg.glow_size", vec![
                                (0.0, 1.0),
                                (1.0, 1.0)
                            ]),
                            AnimTrack::vec4("icon.color", vec![
                                (0.0, color("#0")),
                                (1.0, color("#f")),
                            ]),
                        ]
                    ) 
                ]
            ),
            bg:Quad{
                shader_id:cx.add_shader(bg_sh),
                ..Style::style(cx)
            },
            text:Text{..Style::style(cx)},
            event:ButtonEvent::None
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum ButtonEvent{
    None,
    Clicked
}

impl Button{
    pub fn def_bg_shader(cx:&mut Cx)->Shader{
        let mut sh = Quad::def_shader(cx);
        sh.add_ast(shader_ast!({

            let border_color:vec4<Instance>;
            let glow_size:float<Instance>;

            const glow_color:vec4 = color("#30f");
            const border_radius:float = 6.5;
            const border_width:float = 1.0;

            fn pixel()->vec4{
                df_viewport(pos * vec2(w, h));
                df_box(0., 0., w, h, border_radius);
                df_shape += 3.;
                df_fill_keep(color);
                df_stroke_keep(border_color, border_width);
                df_blur = 2.;
                return df_glow(glow_color, glow_size);
            }
        }));
        sh
    }

    pub fn handle(&mut self, cx:&mut Cx, event:&Event)->ButtonEvent{
        match event.hits(self.bg_area, cx){
            Event::Animate(ae)=>{
                self.anim.calc(cx, "bg.color", ae.time, self.bg_area);
                self.anim.calc(cx, "bg.border_color", ae.time, self.bg_area);
                self.anim.calc(cx, "bg.glow_size", ae.time, self.bg_area);
            },
            Event::FingerDown(_fe)=>{
                self.event = ButtonEvent::Clicked;
                self.anim.change_state(cx, ButtonState::Down);
            },
            Event::FingerMove(_fe)=>{
            },
            _=>{
                 self.event = ButtonEvent::None
            }
        };
        self.event.clone()
   }

    pub fn draw_with_label(&mut self, cx:&mut Cx, label: &str){

        // pull the bg color from our animation system, uses 'default' value otherwise
        self.bg.color = self.anim.last_vec4("bg.color");
        self.bg_area = self.bg.begin(cx, &self.bg_layout);
        // push the 2 vars we added to bg shader
        self.anim.last_push(cx, "bg.border_color", self.bg_area);
        self.anim.last_push(cx, "bg.glow_size", self.bg_area);

        self.text.draw_text(cx, Computed, Computed, label);
        
        self.bg.end(cx);

        self.anim.set_area(cx, self.bg_area); // if our area changed, update animation
    }
}
